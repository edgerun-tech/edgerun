import { createSignal, onMount, onCleanup, Show, For, createEffect } from 'solid-js';
import { Peer } from 'peerjs';
import type { DataConnection } from 'peerjs';
import { BiRegularVideo, BiRegularVideoOff, BiRegularMicrophone, BiRegularMicrophoneOff, BiRegularPhoneOff, BiRegularSend, BiRegularX } from 'solid-icons/bi';

type Message = {
  id: string;
  from: string;
  text: string;
  time: number;
  isLocal: boolean;
};

export default function CallApp() {
  let localVideoRef: HTMLVideoElement | undefined;
  let remoteVideoRef: HTMLVideoElement | undefined;
  let messagesEndRef: HTMLDivElement | undefined;
  
  const [peerId, setPeerId] = createSignal<string | null>(null);
  const [callId, setCallId] = createSignal<string>('');
  const [connected, setConnected] = createSignal(false);
  const [callConnected, setCallConnected] = createSignal(false);
  const [videoEnabled, setVideoEnabled] = createSignal(true);
  const [audioEnabled, setAudioEnabled] = createSignal(true);
  const [status, setStatus] = createSignal('Initializing...');
  const [error, setError] = createSignal<string | null>(null);
  const [currentCall, setCurrentCall] = createSignal<any>(null);
  const [messages, setMessages] = createSignal<Message[]>([]);
  const [newMessage, setNewMessage] = createSignal('');
  const [connections, setConnections] = createSignal<DataConnection[]>([]);
  const [showChat, setShowChat] = createSignal(false);
  
  let peer: Peer | null = null;
  let localStream: MediaStream | null = null;

  const generateCallId = () => {
    const id = Math.random().toString(36).substring(2, 10);
    setCallId(id);
    return id;
  };

  const initPeer = async () => {
    setStatus('Connecting to signaling server...');
    
    try {
      peer = new Peer({
        debug: 1
      });

      peer.on('open', (id) => {
        setPeerId(id);
        setConnected(true);
        setStatus('Ready - share your link or enter a call ID');
      });

      peer.on('connection', (conn) => {
        setStatus('Peer connected!');
        setupDataConnection(conn);
        
        conn.on('open', () => {
          setConnections(prev => [...prev, conn]);
          updateConnectionStatus();
        });
      });

      peer.on('call', async (call) => {
        setStatus('Incoming call...');
        
        try {
          const stream = await getLocalStream();
          call.answer(stream);
          
          call.on('stream', (remoteStream) => {
            setCallConnected(true);
            setStatus('Connected!');
            if (remoteVideoRef) {
              remoteVideoRef.srcObject = remoteStream;
            }
          });

          call.on('close', () => {
            setCallConnected(false);
            setStatus('Call ended');
            if (remoteVideoRef) {
              remoteVideoRef.srcObject = null;
            }
          });

          setCurrentCall(call);
        } catch (e) {
          console.error('Failed to answer call:', e);
          setError('Failed to answer call. Please check camera/microphone permissions.');
          setStatus('Failed to answer call');
        }
      });

      peer.on('error', (err) => {
        console.error('Peer error:', err);
        setError(`Connection error: ${err.type}`);
        setStatus('Connection error');
      });

    } catch (e) {
      console.error('Failed to create peer:', e);
      setStatus('Failed to initialize');
    }
  };

  const getLocalStream = async () => {
    if (!localStream) {
      localStream = await navigator.mediaDevices.getUserMedia({ 
        video: true, 
        audio: true 
      });
    }
    if (localVideoRef) {
      localVideoRef.srcObject = localStream;
    }
    return localStream;
  };

  const startCall = async () => {
    if (!peer || !callId()) return;
    
    setStatus('Calling...');
    
    try {
      const stream = await getLocalStream();
      const call = peer.call(callId(), stream);
      
      const dataConn = peer.connect(callId());
      dataConn.on('open', () => {
        setConnections(prev => [...prev, dataConn]);
        updateConnectionStatus();
      });
      setupDataConnection(dataConn);
      
      call.on('stream', (remoteStream) => {
        setCallConnected(true);
        setStatus('Connected!');
        if (remoteVideoRef) {
          remoteVideoRef.srcObject = remoteStream;
        }
      });

      call.on('close', () => {
        setCallConnected(false);
        setStatus('Call ended');
      });

      call.on('error', (err) => {
        console.error('Call error:', err);
        setStatus('Call failed');
      });

      setCurrentCall(call);
    } catch (e) {
      console.error('Failed to start call:', e);
      setError('Failed to start call. Please check camera/microphone permissions.');
      setStatus('Failed to start call');
    }
  };

  const endCall = () => {
    if (currentCall()) {
      currentCall().close();
      setCurrentCall(null);
    }
    setCallConnected(false);
    setStatus('Ready');
    if (remoteVideoRef) {
      remoteVideoRef.srcObject = null;
    }
  };

  const toggleVideo = () => {
    if (localStream) {
      localStream.getVideoTracks().forEach(track => {
        track.enabled = !videoEnabled();
      });
      setVideoEnabled(!videoEnabled());
    }
  };

  const toggleAudio = () => {
    if (localStream) {
      localStream.getAudioTracks().forEach(track => {
        track.enabled = !audioEnabled();
      });
      setAudioEnabled(!audioEnabled());
    }
  };

  const copyLink = () => {
    const link = `${window.location.origin}/call/${peerId()}`;
    navigator.clipboard.writeText(link);
    setStatus('Link copied to clipboard!');
  };

  const setupDataConnection = (conn: DataConnection) => {
    conn.on('data', (data: any) => {
      if (data.type === 'message') {
        const msg: Message = {
          id: Date.now().toString(),
          from: conn.peer,
          text: data.text,
          time: data.time,
          isLocal: false
        };
        setMessages(prev => [...prev, msg]);
      }
    });

    conn.on('close', () => {
      setConnections(prev => prev.filter(c => c.peer !== conn.peer));
      updateConnectionStatus();
    });
  };

  const updateConnectionStatus = () => {
    const count = connections().length;
    if (count === 0) {
      setStatus(callConnected() ? 'Connected (video only)' : 'Ready - share your link or enter a call ID');
    } else if (count === 1) {
      setStatus(callConnected() ? 'Connected (1 peer + chat)' : '1 peer connected');
    } else {
      setStatus(`${count} peers connected`);
    }
  };

  const sendMessage = () => {
    const text = newMessage().trim();
    if (!text) return;

    const msg: Message = {
      id: Date.now().toString(),
      from: peerId() || 'me',
      text,
      time: Date.now(),
      isLocal: true
    };

    setMessages(prev => [...prev, msg]);

    connections().forEach(conn => {
      conn.send({ type: 'message', text, time: msg.time });
    });

    setNewMessage('');
  };

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  const joinFromUrl = () => {
    const path = window.location.pathname;
    const match = path.match(/\/call\/(.+)/);
    if (match && peerId()) {
      const targetId = match[1];
      if (targetId === peerId()) return;
      setCallId(targetId);
      setStatus('Click "Connect" to join the call');
    }
  };

  const connectChat = () => {
    if (!peer || !callId()) return;
    if (callId() === peerId()) return;
    
    setStatus('Connecting...');
    const dataConn = peer.connect(callId());
    dataConn.on('open', () => {
      setConnections(prev => [...prev, dataConn]);
      updateConnectionStatus();
      setStatus('Connected for chat!');
    });
    setupDataConnection(dataConn);
    dataConn.on('error', (err) => {
      console.error('Connection error:', err);
      setStatus('Connection failed');
    });
  };

  onMount(async () => {
    await initPeer();
    joinFromUrl();
  });

  createEffect(() => {
    messages();
    if (messagesEndRef) {
      messagesEndRef.scrollIntoView({ behavior: 'smooth' });
    }
  });

  onCleanup(() => {
    if (localStream) {
      for (const track of localStream.getTracks()) {
        track.stop();
      }
    }
    for (const conn of connections()) {
      conn.close();
    }
    if (peer) {
      peer.destroy();
    }
  });

  return (
    <div class="h-full flex flex-col bg-[#1a1a1a] text-neutral-200 p-4">
      {/* Header */}
      <div class="p-4 border-b border-neutral-800 flex items-center justify-between">
        <div>
          <h2 class="text-lg font-semibold text-white">Video Call</h2>
          <Show when={error()} fallback={<p class="text-sm text-neutral-400">{status()}</p>}>
            <div class="flex items-center gap-2">
              <p class="text-sm text-red-400">{error()}</p>
              <button
                type="button"
                onClick={() => {
                  setError(null);
                  initPeer();
                }}
                class="text-xs px-2 py-1 bg-red-600 hover:bg-red-500 text-white rounded transition-colors"
              >
                Retry
              </button>
            </div>
          </Show>
        </div>
        <div class="flex items-center gap-2">
          <Show when={connections().length > 0 || callConnected()}>
            <span class="text-xs px-2 py-1 bg-green-900 text-green-300 rounded-full">
              {connections().length + (callConnected() ? 1 : 0)} connected
            </span>
          </Show>
          <button
            type="button"
            onClick={() => setShowChat(!showChat())}
            class={`p-2 rounded-lg transition-colors ${
              showChat() ? 'bg-blue-600 text-white' : 'bg-neutral-700 text-neutral-300 hover:bg-neutral-600'
            }`}
          >
            <BiRegularSend size={18} />
          </button>
        </div>
      </div>

      {/* Chat Panel */}
      <Show when={showChat()}>
        <div class="border-b border-neutral-800 flex flex-col" style="height: 200px;">
          <div class="flex-1 overflow-y-auto p-3 space-y-2">
            <Show when={messages().length === 0}>
              <p class="text-sm text-neutral-500 text-center">No messages yet</p>
            </Show>
            <For each={messages()}>
              {(msg) => (
                <div class={`flex ${msg.isLocal ? 'justify-end' : 'justify-start'}`}>
                  <div class={`max-w-[80%] px-3 py-2 rounded-lg ${
                    msg.isLocal ? 'bg-blue-600 text-white' : 'bg-neutral-700 text-neutral-200'
                  }`}>
                    <p class="text-sm">{msg.text}</p>
                    <p class="text-xs opacity-60 mt-1">
                      {msg.isLocal ? 'You' : msg.from.slice(0, 8)} · {new Date(msg.time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                    </p>
                  </div>
                </div>
              )}
            </For>
            <div ref={messagesEndRef} />
          </div>
          <div class="p-3 border-t border-neutral-800 flex gap-2">
            <input
              type="text"
              value={newMessage()}
              onInput={(e) => setNewMessage(e.currentTarget.value)}
              onKeyPress={handleKeyPress}
              placeholder="Type a message..."
              disabled={connections().length === 0}
              class="flex-1 px-3 py-2 bg-neutral-800 border border-neutral-700 rounded-lg text-sm text-white placeholder-neutral-500 focus:outline-none focus:border-blue-500 disabled:opacity-50"
            />
            <button
              type="button"
              onClick={sendMessage}
              disabled={connections().length === 0 || !newMessage().trim()}
              class="p-2 bg-blue-600 hover:bg-blue-500 rounded-lg text-white transition-colors disabled:opacity-50"
            >
              <BiRegularSend size={18} />
            </button>
          </div>
        </div>
      </Show>

      {/* Video Grid */}
      <div class="flex-1 flex gap-2 p-4">
        {/* Remote Video */}
        <div class="flex-1 bg-black rounded-lg overflow-hidden relative">
          <Show when={callConnected()} fallback={
            <div class="absolute inset-0 flex items-center justify-center text-neutral-500">
              No remote video
            </div>
          }>
            <video 
              ref={remoteVideoRef} 
              autoplay 
              playsinline 
              class="w-full h-full object-cover"
            >
              <track kind="captions" />
            </video>
          </Show>
        </div>
        
        {/* Local Video */}
        <div class="w-32 h-24 bg-black rounded-lg overflow-hidden">
          <video 
            ref={localVideoRef} 
            autoplay 
            playsinline 
            muted
            class="w-full h-full object-cover"
          >
            <track kind="captions" />
          </video>
        </div>
      </div>

      {/* Controls */}
      <div class="p-4 flex items-center justify-center gap-4 border-t border-neutral-800">
        <button
          type="button"
          onClick={toggleAudio}
          class={`p-3 rounded-full transition-colors ${
            audioEnabled() ? 'bg-neutral-700 hover:bg-neutral-600 text-white' : 'bg-red-600 text-white'
          }`}
        >
          {audioEnabled() ? <BiRegularMicrophone size={20} /> : <BiRegularMicrophoneOff size={20} />}
        </button>
        
        <button
          type="button"
          onClick={toggleVideo}
          class={`p-3 rounded-full transition-colors ${
            videoEnabled() ? 'bg-neutral-700 hover:bg-neutral-600 text-white' : 'bg-red-600 text-white'
          }`}
        >
          {videoEnabled() ? <BiRegularVideo size={20} /> : <BiRegularVideoOff size={20} />}
        </button>

        <Show when={!callConnected()} fallback={
          <button
            type="button"
            onClick={endCall}
            class="p-3 rounded-full bg-red-600 hover:bg-red-500 text-white transition-colors"
          >
            <BiRegularPhoneOff size={20} />
          </button>
        }>
          <button
            type="button"
            onClick={connectChat}
            disabled={!callId() || !peerId() || callId() === peerId()}
            class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-sm transition-colors disabled:opacity-50"
          >
            Connect
          </button>
          <button
            type="button"
            onClick={startCall}
            disabled={!callId() || !peerId() || callId() === peerId()}
            class="p-3 rounded-full bg-green-600 hover:bg-green-500 text-white transition-colors disabled:opacity-50"
          >
            <BiRegularVideo size={20} />
          </button>
        </Show>
      </div>

      {/* Call ID Input */}
      <div class="p-4 border-t border-neutral-800 space-y-3">
        <div class="flex gap-2">
          <input
            type="text"
            value={callId()}
            onInput={(e) => setCallId(e.currentTarget.value)}
            placeholder="Enter call ID to join"
            class="flex-1 px-3 py-2 bg-neutral-800 border border-neutral-700 rounded-lg text-sm text-white placeholder-neutral-500 focus:outline-none focus:border-blue-500"
          />
        </div>
        
        <Show when={peerId()}>
          <div class="flex gap-2">
            <button
              type="button"
              onClick={copyLink}
              class="flex-1 px-3 py-2 bg-blue-600 hover:bg-blue-500 rounded-lg text-sm text-white transition-colors"
            >
              Copy My Link
            </button>
            <button
              type="button"
              onClick={generateCallId}
              class="px-3 py-2 bg-neutral-700 hover:bg-neutral-600 rounded-lg text-sm text-white transition-colors"
            >
              New Call
            </button>
            <Show when={connections().length > 0 && !callConnected()}>
              <button
                type="button"
                onClick={() => {
                  for (const conn of connections()) {
                    conn.close();
                  }
                  setConnections([]);
                  updateConnectionStatus();
                }}
                class="px-3 py-2 bg-red-600 hover:bg-red-500 rounded-lg text-sm text-white transition-colors"
              >
                Disconnect
              </button>
            </Show>
          </div>
          <p class="text-xs text-neutral-500 text-center">
            Your ID: <span class="font-mono">{peerId()}</span>
          </p>
        </Show>
      </div>
    </div>
  );
}
