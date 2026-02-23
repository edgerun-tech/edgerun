/**
 * Activity Feed Component
 * Shows recent actions, deployments, and events
 */

import { createSignal, createEffect, For, Show, onMount } from 'solid-js';
import { Motion } from 'solid-motionone';
import { TbOutlineClock, TbOutlineGitCommit, TbOutlineCloudUpload, TbOutlineFile, TbOutlineTerminal } from 'solid-icons/tb';

type ActivityType = 'command' | 'deployment' | 'file' | 'git' | 'system';

interface Activity {
  id: string;
  type: ActivityType;
  title: string;
  description: string;
  timestamp: Date;
  icon: any;
  color: string;
}

const [activities, setActivities] = createSignal<Activity[]>([]);

// Add activity to feed
export function addActivity(activity: Omit<Activity, 'id' | 'timestamp'>) {
  const newActivity: Activity = {
    ...activity,
    id: `activity-${Date.now()}-${Math.random().toString(36).slice(2)}`,
    timestamp: new Date(),
  };
  setActivities(prev => [newActivity, ...prev].slice(0, 50)); // Keep last 50
}

// Load activities from localStorage
function loadActivities(): Activity[] {
  if (typeof window === 'undefined') return [];
  try {
    const stored = localStorage.getItem('browser-os-activities');
    if (stored) {
      const parsed = JSON.parse(stored);
      return parsed.map((a: any) => ({
        ...a,
        timestamp: new Date(a.timestamp),
      }));
    }
  } catch {}
  return [];
}

// Save activities to localStorage
function saveActivities(acts: Activity[]) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem('browser-os-activities', JSON.stringify(acts));
  } catch {}
}

export default function ActivityFeed() {
  const [filter, setFilter] = createSignal<ActivityType | 'all'>('all');
  const [showAll, setShowAll] = createSignal(false);

  onMount(() => {
    const loaded = loadActivities();
    if (loaded.length === 0) {
      // Add some sample activities for demo
      addActivity({
        type: 'system',
        title: 'Browser OS Started',
        description: 'Welcome to Browser OS! Your session has begun.',
        icon: TbOutlineCloudUpload,
        color: 'text-blue-400',
      });
    } else {
      setActivities(loaded);
    }
  });

  createEffect(() => {
    saveActivities(activities());
  });

  const filteredActivities = () => {
    const acts = activities();
    const filtered = filter() === 'all' 
      ? acts 
      : acts.filter(a => a.type === filter());
    
    return showAll() ? filtered : filtered.slice(0, 10);
  };

  const formatTime = (date: Date) => {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);

    if (minutes < 1) return 'Just now';
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    if (days < 7) return `${days}d ago`;
    return date.toLocaleDateString();
  };

  const activityTypes: { value: ActivityType | 'all'; label: string; color: string }[] = [
    { value: 'all', label: 'All', color: 'text-neutral-400' },
    { value: 'command', label: 'Commands', color: 'text-green-400' },
    { value: 'deployment', label: 'Deployments', color: 'text-blue-400' },
    { value: 'file', label: 'Files', color: 'text-yellow-400' },
    { value: 'git', label: 'Git', color: 'text-purple-400' },
    { value: 'system', label: 'System', color: 'text-neutral-400' },
  ];

  return (
    <div class="h-full flex flex-col bg-[#1a1a1a] text-neutral-200">
      {/* Header */}
      <div class="p-4 border-b border-neutral-800">
        <h2 class="text-lg font-semibold text-white flex items-center gap-2">
          <TbOutlineClock size={20} />
          Activity Feed
        </h2>
      </div>

      {/* Filter Tabs */}
      <div class="flex gap-1 p-2 border-b border-neutral-800 overflow-x-auto">
        <For each={activityTypes}>
          {(type) => (
            <button
              type="button"
              onClick={() => setFilter(type.value)}
              class={`px-3 py-1.5 rounded-lg text-sm whitespace-nowrap transition-colors ${
                filter() === type.value
                  ? 'bg-neutral-700 text-white'
                  : 'text-neutral-400 hover:text-white hover:bg-neutral-800'
              }`}
            >
              {type.label}
            </button>
          )}
        </For>
      </div>

      {/* Activity List */}
      <div class="flex-1 overflow-y-auto p-4">
        <Show 
          when={filteredActivities().length > 0}
          fallback={
            <div class="flex flex-col items-center justify-center h-full text-neutral-500">
              <TbOutlineClock size={48} class="mb-4 opacity-50" />
              <p>No activity yet</p>
            </div>
          }
        >
          <div class="space-y-3">
            <For each={filteredActivities()}>
              {(activity) => (
                <Motion.div
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  class="flex gap-3 p-3 rounded-lg bg-neutral-800/50 hover:bg-neutral-800 transition-colors"
                >
                  <div class={`flex-shrink-0 w-10 h-10 rounded-lg bg-neutral-900 flex items-center justify-center ${activity.color}`}>
                    <activity.icon size={20} />
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2">
                      <span class="font-medium text-white truncate">{activity.title}</span>
                      <span class="text-xs text-neutral-500">{formatTime(activity.timestamp)}</span>
                    </div>
                    <p class="text-sm text-neutral-400 mt-1">{activity.description}</p>
                  </div>
                </Motion.div>
              )}
            </For>
          </div>
        </Show>
      </div>

      {/* Show More */}
      <Show when={activities().length > 10}>
        <div class="p-4 border-t border-neutral-800">
          <button
            type="button"
            onClick={() => setShowAll(!showAll())}
            class="w-full py-2 px-4 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 transition-colors"
          >
            {showAll() ? 'Show Less' : `Show All (${activities().length})`}
          </button>
        </div>
      </Show>

      {/* Clear History */}
      <Show when={activities().length > 0}>
        <div class="p-4 border-t border-neutral-800">
          <button
            type="button"
            onClick={() => setActivities([])}
            class="w-full py-2 px-4 rounded-lg bg-red-900/20 hover:bg-red-900/30 text-red-400 transition-colors text-sm"
          >
            Clear History
          </button>
        </div>
      </Show>
    </div>
  );
}
