/**
 * Pluggable pub/sub bus, mirroring python/src/d3rac_agents/bus.py exactly (same JSONL
 * wire format: {topic, ts, payload}), so a Python agent's publish is directly readable
 * by a Node agent's subscribe and vice versa on the file transport.
 */
import * as fs from 'fs';
import * as path from 'path';
import { BUS_FILE_DIR, BUS_TRANSPORT, BUS_REDIS_URL } from '../config/generatedManifest';

export interface BusEvent<T = Record<string, unknown>> {
  topic: string;
  ts: number;
  payload: T;
}

export interface Bus {
  publish<T>(topic: string, payload: T): Promise<void>;
  readAll<T>(topic: string): Promise<BusEvent<T>[]>;
}

// dist/bus/messageBus.js -> ../../.. == agents/ (the package root shared with python/).
// Used to anchor relative bus paths so they don't depend on the process's current working
// directory, which differs between `python3 -m d3rac_agents.cli` (run from agents/) and
// `npm run start` (run from agents/node/) in the Makefile's `make dev`.
const AGENTS_ROOT = path.resolve(__dirname, '../../..');

export class FileBus implements Bus {
  private dir: string;

  constructor(baseDir?: string) {
    const raw = baseDir ?? BUS_FILE_DIR;
    this.dir = path.isAbsolute(raw) ? raw : path.resolve(AGENTS_ROOT, raw);
    fs.mkdirSync(this.dir, { recursive: true });
  }

  private topicPath(topic: string): string {
    return path.join(this.dir, `${topic.replace(/\//g, '_')}.jsonl`);
  }

  async publish<T>(topic: string, payload: T): Promise<void> {
    const event: BusEvent<T> = { topic, ts: Date.now() / 1000, payload };
    fs.appendFileSync(this.topicPath(topic), JSON.stringify(event) + '\n');
  }

  async readAll<T>(topic: string): Promise<BusEvent<T>[]> {
    const file = this.topicPath(topic);
    if (!fs.existsSync(file)) return [];
    return fs
      .readFileSync(file, 'utf-8')
      .split('\n')
      .filter((line) => line.trim().length > 0)
      .map((line) => JSON.parse(line) as BusEvent<T>);
  }
}

/**
 * Production transport. Requires the `redis` npm package (add it to node/package.json
 * dependencies) and a running Redis instance at BUS_REDIS_URL.
 */
export class RedisBus implements Bus {
  private client: unknown;

  constructor(url?: string) {
    // Lazy require so FileBus users aren't forced to install `redis`.
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const { createClient } = require('redis');
    this.client = createClient({ url: url ?? BUS_REDIS_URL });
  }

  async publish<T>(topic: string, payload: T): Promise<void> {
    const event = { topic, ts: Date.now() / 1000, payload };
    // @ts-expect-error - untyped lazy-loaded client
    await this.client.rPush(topic, JSON.stringify(event));
  }

  async readAll<T>(topic: string): Promise<BusEvent<T>[]> {
    // @ts-expect-error - untyped lazy-loaded client
    const raw: string[] = await this.client.lRange(topic, 0, -1);
    return raw.map((r) => JSON.parse(r));
  }
}

export function getBus(): Bus {
  return BUS_TRANSPORT === 'redis' ? new RedisBus() : new FileBus();
}
