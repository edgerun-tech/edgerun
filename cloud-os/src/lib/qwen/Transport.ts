export interface Transport {
  start(): Promise<void>;
  close(): Promise<void>;
  waitForExit(): Promise<void>;
  write(message: string): void;
  readMessages(): AsyncGenerator<unknown, void, unknown>;
  readonly isReady: boolean;
  readonly exitErrorValue: Error | null;
}
