import { createSignal, onMount, onCleanup, Show, For, createEffect } from "solid-js";
import { Peer } from "peerjs";
import { BiRegularVideo, BiRegularVideoOff, BiRegularMicrophone, BiRegularMicrophoneOff, BiRegularPhoneOff, BiRegularSend } from "solid-icons/bi";
import { TbOutlineBellRinging, TbOutlinePhone, TbOutlineVideo } from "solid-icons/tb";
import { UI_EVENT_TOPICS } from "../../lib/ui-intents";
import { subscribeEvent } from "../../stores/eventbus";

function createPeerClient(options) {
  if (typeof window !== "undefined" && typeof window.__intentUiPeerFactory === "function") {
    return window.__intentUiPeerFactory(options || {});
  }
  return new Peer(options || {});
}

function CallApp() {
  let localVideoRef;
  let remoteVideoRef;
  let messagesEndRef;
  const [peerId, setPeerId] = createSignal(null);
  const [callId, setCallId] = createSignal("");
  const [connected, setConnected] = createSignal(false);
  const [callConnected, setCallConnected] = createSignal(false);
  const [videoEnabled, setVideoEnabled] = createSignal(true);
  const [audioEnabled, setAudioEnabled] = createSignal(true);
  const [status, setStatus] = createSignal("Initializing...");
  const [error, setError] = createSignal(null);
  const [currentCall, setCurrentCall] = createSignal(null);
  const [messages, setMessages] = createSignal([]);
  const [newMessage, setNewMessage] = createSignal("");
  const [connections, setConnections] = createSignal([]);
  const [showChat, setShowChat] = createSignal(false);
  const [ringing, setRinging] = createSignal(false);
  const [ringTarget, setRingTarget] = createSignal("");
  const [ringMode, setRingMode] = createSignal("call");
  const [autoJoinTarget, setAutoJoinTarget] = createSignal("");
  const [autoJoinAttemptKey, setAutoJoinAttemptKey] = createSignal("");
  let handleIntentCallRing;
  let unsubscribeCallRing;
  let peer = null;
  let localStream = null;
  let ringTimeout = null;
  const generateCallId = () => {
    const id = Math.random().toString(36).substring(2, 10);
    setCallId(id);
    return id;
  };
  const initPeer = async () => {
    setStatus("Connecting to signaling server...");
    try {
      peer = createPeerClient({
        debug: 1
      });
      peer.on("open", (id) => {
        setPeerId(id);
        setConnected(true);
        setStatus("Ready - share your link or enter a call ID");
      });
      peer.on("connection", (conn) => {
        setStatus("Peer connected!");
        setupDataConnection(conn);
        conn.on("open", () => {
          setConnections((prev) => [...prev, conn]);
          updateConnectionStatus();
        });
      });
      peer.on("call", async (call) => {
        setStatus("Incoming call...");
        try {
          const stream = await getLocalStream();
          call.answer(stream);
          call.on("stream", (remoteStream) => {
            setCallConnected(true);
            setStatus("Connected!");
            if (remoteVideoRef) {
              remoteVideoRef.srcObject = remoteStream;
            }
          });
          call.on("close", () => {
            setCallConnected(false);
            setStatus("Call ended");
            if (remoteVideoRef) {
              remoteVideoRef.srcObject = null;
            }
          });
          setCurrentCall(call);
        } catch (e) {
          console.error("Failed to answer call:", e);
          setError("Failed to answer call. Please check camera/microphone permissions.");
          setStatus("Failed to answer call");
        }
      });
      peer.on("error", (err) => {
        console.error("Peer error:", err);
        setError(`Connection error: ${err.type}`);
        setStatus("Connection error");
      });
    } catch (e) {
      console.error("Failed to create peer:", e);
      setStatus("Failed to initialize");
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
  const startCall = async (targetOverride = "") => {
    const target = String(targetOverride || callId() || "").trim();
    if (!peer || !target) return;
    if (target === peerId()) return;
    setCallId(target);
    setStatus("Calling...");
    try {
      const stream = await getLocalStream();
      const call = peer.call(target, stream);
      const dataConn = peer.connect(target);
      dataConn.on("open", () => {
        setConnections((prev) => [...prev, dataConn]);
        updateConnectionStatus();
      });
      setupDataConnection(dataConn);
      call.on("stream", (remoteStream) => {
        setCallConnected(true);
        setStatus("Connected!");
        if (remoteVideoRef) {
          remoteVideoRef.srcObject = remoteStream;
        }
      });
      call.on("close", () => {
        setCallConnected(false);
        setStatus("Call ended");
      });
      call.on("error", (err) => {
        console.error("Call error:", err);
        setStatus("Call failed");
      });
      setCurrentCall(call);
    } catch (e) {
      console.error("Failed to start call:", e);
      setError("Failed to start call. Please check camera/microphone permissions.");
      setStatus("Failed to start call");
    }
  };
  const endCall = () => {
    if (currentCall()) {
      currentCall().close();
      setCurrentCall(null);
    }
    setCallConnected(false);
    setStatus("Ready");
    if (remoteVideoRef) {
      remoteVideoRef.srcObject = null;
    }
  };
  const toggleVideo = () => {
    if (localStream) {
      localStream.getVideoTracks().forEach((track) => {
        track.enabled = !videoEnabled();
      });
      setVideoEnabled(!videoEnabled());
    }
  };
  const toggleAudio = () => {
    if (localStream) {
      localStream.getAudioTracks().forEach((track) => {
        track.enabled = !audioEnabled();
      });
      setAudioEnabled(!audioEnabled());
    }
  };
  const copyLink = () => {
    const link = `${window.location.origin}/call/${peerId()}`;
    navigator.clipboard.writeText(link);
    setStatus("Link copied to clipboard!");
  };
  const setupDataConnection = (conn) => {
    conn.on("data", (data) => {
      if (data.type === "message") {
        const msg = {
          id: Date.now().toString(),
          from: conn.peer,
          text: data.text,
          time: data.time,
          isLocal: false
        };
        setMessages((prev) => [...prev, msg]);
      }
    });
    conn.on("close", () => {
      setConnections((prev) => prev.filter((c) => c.peer !== conn.peer));
      updateConnectionStatus();
    });
  };
  const updateConnectionStatus = () => {
    const count = connections().length;
    if (count === 0) {
      setStatus(callConnected() ? "Connected (video only)" : "Ready - share your link or enter a call ID");
    } else if (count === 1) {
      setStatus(callConnected() ? "Connected (1 peer + chat)" : "1 peer connected");
    } else {
      setStatus(`${count} peers connected`);
    }
  };
  const sendMessage = () => {
    const text = newMessage().trim();
    if (!text) return;
    const msg = {
      id: Date.now().toString(),
      from: peerId() || "me",
      text,
      time: Date.now(),
      isLocal: true
    };
    setMessages((prev) => [...prev, msg]);
    connections().forEach((conn) => {
      conn.send({ type: "message", text, time: msg.time });
    });
    setNewMessage("");
  };
  const handleKeyPress = (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };
  const joinFromUrl = () => {
    const path = window.location.pathname;
    const match = path.match(/\/call\/(.+)/);
    if (match) {
      const targetId = decodeURIComponent(match[1] || "").trim();
      if (!targetId) return;
      setCallId(targetId);
      setAutoJoinTarget(targetId);
      setStatus("Joining call from shared link...");
    }
  };
  const connectChat = () => {
    if (!peer || !callId()) return;
    if (callId() === peerId()) return;
    setStatus("Connecting...");
    const dataConn = peer.connect(callId());
    dataConn.on("open", () => {
      setConnections((prev) => [...prev, dataConn]);
      updateConnectionStatus();
      setStatus("Connected for chat!");
    });
    setupDataConnection(dataConn);
    dataConn.on("error", (err) => {
      console.error("Connection error:", err);
      setStatus("Connection failed");
    });
  };
  const stopRinging = () => {
    if (ringTimeout) {
      clearTimeout(ringTimeout);
      ringTimeout = null;
    }
    setRinging(false);
  };
  const startRinging = (target, mode2) => {
    stopRinging();
    setRingTarget(target || "Contact");
    setRingMode(mode2 === "video" ? "video" : "call");
    setStatus(`Ringing ${target || "contact"}...`);
    setRinging(true);
    ringTimeout = setTimeout(() => {
      setRinging(false);
      setStatus("No answer yet");
    }, 1e4);
  };
  onMount(async () => {
    await initPeer();
    joinFromUrl();
    handleIntentCallRing = (event) => {
      const detail = event?.payload || {};
      startRinging(detail.contact, detail.mode);
    };
    unsubscribeCallRing = subscribeEvent(UI_EVENT_TOPICS.action.callRinging, handleIntentCallRing);
  });
  createEffect(() => {
    const target = autoJoinTarget();
    const self = peerId();
    if (!target || !self) return;
    if (target === self) return;
    const key = `${self}->${target}`;
    if (autoJoinAttemptKey() === key) return;
    if (currentCall() || callConnected()) return;
    setAutoJoinAttemptKey(key);
    void startCall(target);
  });
  createEffect(() => {
    messages();
    if (messagesEndRef) {
      messagesEndRef.scrollIntoView({ behavior: "smooth" });
    }
  });
  const connectionCount = () => connections().length + (callConnected() ? 1 : 0);
  const statusTone = () => {
    if (error()) return "text-destructive bg-destructive/15 border-destructive/40";
    if (callConnected()) return "text-primary bg-primary/20 border-primary/40";
    if (connected()) return "text-primary bg-primary/20 border-primary/40";
    return "text-muted-foreground bg-muted/60 border-border";
  };
  onCleanup(() => {
    if (unsubscribeCallRing) unsubscribeCallRing();
    stopRinging();
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
  createEffect(() => {
    if (callConnected()) {
      stopRinging();
    }
  });
  return <div class="relative h-full overflow-hidden rounded-xl border border-border bg-gradient-to-br from-background via-card to-muted text-foreground shadow-2xl">
      <div class="pointer-events-none absolute inset-0 opacity-40" style={{
      background: "radial-gradient(circle at 15% 10%, rgba(56,189,248,0.22), transparent 38%), radial-gradient(circle at 85% 0%, rgba(59,130,246,0.2), transparent 42%)"
    }} />
      <div class="relative z-10 flex h-full flex-col">
        <div class="flex items-center justify-between border-b border-border px-4 py-3">
          <div>
            <h2 class="text-lg font-semibold tracking-tight text-foreground">Call Studio</h2>
            <p class="mt-0.5 text-xs text-muted-foreground">{status()}</p>
          </div>
          <div class="flex items-center gap-2">
            <span class={`rounded-full border px-2.5 py-1 text-xs font-medium ${statusTone()}`}>
              {error() ? "Issue detected" : callConnected() ? "Live call" : connected() ? "Online" : "Booting"}
            </span>
            <Show when={connectionCount() > 0}>
              <span class="rounded-full border border-primary/40 bg-primary/20 px-2.5 py-1 text-xs text-primary">
                {connectionCount()} connected
              </span>
            </Show>
            <button
    type="button"
    onClick={() => setShowChat(!showChat())}
    class={`rounded-lg border px-2.5 py-2 transition-colors ${showChat() ? "border-primary/40 bg-primary/20 text-primary" : "border-border bg-secondary text-secondary-foreground hover:bg-accent hover:text-accent-foreground"}`}
    title="Toggle chat panel"
  >
              <BiRegularSend size={18} />
            </button>
          </div>
        </div>

        <Show when={ringing()}>
          <div class="mx-4 mt-3 rounded-xl border border-primary/40 bg-primary/10 px-4 py-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div class="relative flex h-8 w-8 items-center justify-center rounded-full border border-primary/50 bg-primary/20 text-primary">
                  <TbOutlineBellRinging size={16} class="animate-pulse" />
                  <span class="absolute inset-0 rounded-full border border-primary/40 animate-ping" />
                </div>
                <div>
                  <p class="text-sm font-medium text-foreground">
                    Ringing {ringTarget()}
                  </p>
                  <p class="text-xs text-muted-foreground capitalize">
                    {ringMode()} call request in progress
                  </p>
                </div>
              </div>
              <button
    type="button"
    onClick={stopRinging}
    class="rounded-md border border-border bg-secondary px-2.5 py-1.5 text-xs text-secondary-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
  >
                Cancel
              </button>
            </div>
          </div>
        </Show>

        <Show when={error()}>
          <div class="mx-4 mt-3 flex items-center justify-between rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2">
            <p class="text-sm text-destructive">{error()}</p>
            <button
    type="button"
    onClick={() => {
      setError(null);
      initPeer();
    }}
    class="rounded-md border border-border bg-secondary px-2.5 py-1 text-xs font-medium text-secondary-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
  >
              Retry
            </button>
          </div>
        </Show>

        <Show when={showChat()}>
          <div class="mx-4 mt-3 flex h-52 flex-col overflow-hidden rounded-xl border border-border bg-background/60 backdrop-blur-sm">
            <div class="flex-1 space-y-2 overflow-y-auto px-3 py-3">
              <Show when={messages().length === 0}>
                <p class="pt-6 text-center text-sm text-muted-foreground">No messages yet</p>
              </Show>
              <For each={messages()}>
                {(msg) => <div class={`flex ${msg.isLocal ? "justify-end" : "justify-start"}`}>
                    <div class={`max-w-[82%] rounded-xl px-3 py-2 ${msg.isLocal ? "bg-primary text-primary-foreground" : "border border-border bg-secondary text-secondary-foreground"}`}>
                      <p class="text-sm">{msg.text}</p>
                      <p class="mt-1 text-[11px] opacity-70">
                        {msg.isLocal ? "You" : msg.from.slice(0, 8)} · {new Date(msg.time).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                      </p>
                    </div>
                  </div>}
              </For>
              <div ref={messagesEndRef} />
            </div>
            <div class="flex gap-2 border-t border-border px-3 py-2">
              <input
    type="text"
    value={newMessage()}
    onInput={(e) => setNewMessage(e.currentTarget.value)}
    onKeyPress={handleKeyPress}
    placeholder="Type a message..."
    disabled={connections().length === 0}
    class="flex-1 rounded-lg border border-input bg-background px-3 py-2 text-sm text-foreground placeholder-muted-foreground focus:border-ring focus:outline-none disabled:opacity-50"
  />
              <button
    type="button"
    onClick={sendMessage}
    disabled={connections().length === 0 || !newMessage().trim()}
    class="rounded-lg bg-primary px-3 py-2 text-primary-foreground transition-colors hover:opacity-90 disabled:opacity-50"
  >
                <BiRegularSend size={18} />
              </button>
            </div>
          </div>
        </Show>

        <div class="mx-4 my-3 grid min-h-0 flex-1 gap-3 lg:grid-cols-[minmax(0,1fr)_320px]">
          <div class="relative min-h-[300px] overflow-hidden rounded-xl border border-border bg-background/70">
            <Show when={callConnected()} fallback={<div class="absolute inset-0 flex flex-col items-center justify-center gap-3 text-sm text-muted-foreground">
                <div class="rounded-full border border-border bg-card p-3 text-foreground">
                  {ringMode() === "video" ? <TbOutlineVideo size={20} /> : <TbOutlinePhone size={20} />}
                </div>
                <p>{ringing() ? `Trying ${ringTarget()}...` : "Start a call to see remote video"}</p>
              </div>}>
              <video
    ref={remoteVideoRef}
    autoplay
    playsinline
    class="h-full w-full object-cover"
  >
                <track kind="captions" />
              </video>
            </Show>
            <div class="absolute left-3 top-3 rounded-full bg-background/80 px-2.5 py-1 text-xs text-foreground backdrop-blur">
              Remote feed
            </div>
            <div class="absolute bottom-3 right-3 h-24 w-36 overflow-hidden rounded-lg border border-border bg-background shadow-xl">
              <video
    ref={localVideoRef}
    autoplay
    playsinline
    muted
    class="h-full w-full object-cover"
  >
                <track kind="captions" />
              </video>
              <div class="absolute left-1.5 top-1.5 rounded bg-background/80 px-1.5 py-0.5 text-[10px] text-foreground">You</div>
            </div>
          </div>

          <div class="rounded-xl border border-border bg-card/60 p-3">
            <h3 class="text-sm font-semibold text-foreground">Quick Start</h3>
            <ol class="mt-2 space-y-2 text-xs text-muted-foreground">
              <li>1. Share your ID with a contact.</li>
              <li>2. Paste their call ID below.</li>
              <li>3. Press Connect or Start Call.</li>
            </ol>
            <Show when={peerId()}>
              <div class="mt-3 rounded-md border border-border bg-muted/30 p-2 text-xs">
                <p class="text-muted-foreground">Your ID</p>
                <p class="mt-1 truncate font-mono text-foreground">{peerId()}</p>
              </div>
            </Show>
            <div class="mt-3 flex flex-wrap gap-2">
              <button
    type="button"
    onClick={copyLink}
    class="rounded-md border border-border bg-secondary px-2.5 py-1.5 text-xs text-secondary-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
  >
                Copy Link
              </button>
              <button
    type="button"
    onClick={generateCallId}
    class="rounded-md border border-border bg-secondary px-2.5 py-1.5 text-xs text-secondary-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
  >
                New Call ID
              </button>
            </div>
          </div>
        </div>

        <div class="border-t border-border px-4 py-3">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <button
    type="button"
    onClick={toggleAudio}
    class={`rounded-full p-3 transition-colors ${audioEnabled() ? "bg-secondary text-secondary-foreground hover:bg-accent hover:text-accent-foreground" : "bg-destructive text-destructive-foreground hover:opacity-90"}`}
    title={audioEnabled() ? "Mute microphone" : "Unmute microphone"}
  >
                {audioEnabled() ? <BiRegularMicrophone size={20} /> : <BiRegularMicrophoneOff size={20} />}
              </button>
              <button
    type="button"
    onClick={toggleVideo}
    class={`rounded-full p-3 transition-colors ${videoEnabled() ? "bg-secondary text-secondary-foreground hover:bg-accent hover:text-accent-foreground" : "bg-destructive text-destructive-foreground hover:opacity-90"}`}
    title={videoEnabled() ? "Disable camera" : "Enable camera"}
  >
                {videoEnabled() ? <BiRegularVideo size={20} /> : <BiRegularVideoOff size={20} />}
              </button>
              <Show when={callConnected()}>
                <button
    type="button"
    onClick={endCall}
    class="rounded-full bg-destructive p-3 text-destructive-foreground transition-colors hover:opacity-90"
    title="End call"
  >
                  <BiRegularPhoneOff size={20} />
                </button>
              </Show>
            </div>

            <div class="flex min-w-[320px] flex-1 flex-wrap items-center justify-end gap-2">
              <input
    type="text"
    value={callId()}
    onInput={(e) => setCallId(e.currentTarget.value)}
    placeholder="Enter call ID to join"
    class="min-w-[180px] flex-1 rounded-lg border border-input bg-background px-3 py-2 text-sm text-foreground placeholder-muted-foreground focus:border-ring focus:outline-none"
  />
              <button
    type="button"
    onClick={connectChat}
    disabled={!callId() || !peerId() || callId() === peerId()}
    class="rounded-lg bg-secondary px-3.5 py-2 text-sm font-medium text-secondary-foreground transition-colors hover:bg-accent hover:text-accent-foreground disabled:opacity-50"
  >
                Connect
              </button>
              <button
    type="button"
    onClick={startCall}
    disabled={!callId() || !peerId() || callId() === peerId()}
    class="rounded-lg bg-primary px-3.5 py-2 text-sm font-medium text-primary-foreground transition-colors hover:opacity-90 disabled:opacity-50"
  >
                Start Call
              </button>
            </div>
          </div>

          <Show when={connections().length > 0 && !callConnected()}>
            <div class="mt-3">
              <button
    type="button"
    onClick={() => {
      for (const conn of connections()) {
        conn.close();
      }
      setConnections([]);
      updateConnectionStatus();
    }}
    class="rounded-md bg-destructive px-2.5 py-1.5 text-xs text-destructive-foreground transition-colors hover:opacity-90"
  >
                Disconnect All
              </button>
            </div>
          </Show>
        </div>
      </div>
    </div>;
}
export {
  CallApp as default
};
