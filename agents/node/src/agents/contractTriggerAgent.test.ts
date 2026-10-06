import { prepareTronRelease, loadTronConfig } from './contractTriggerAgent';

const cfg = { fullNode: 'https://api.shasta.trongrid.io', controller: 'TXyz', commitmentMap: { 'ng-abuja': 1 } };

function mockCount(n: number) {
  globalThis.fetch = jest.fn().mockResolvedValue({
    ok: true,
    status: 200,
    json: async () => ({ constant_result: [n.toString(16).padStart(64, '0')] }),
  }) as unknown as typeof fetch;
}

describe('prepareTronRelease', () => {
  it('blocks when unconfigured', async () => {
    const r = await prepareTronRelease('ng-abuja', 'Phase 1', 0, 40, { commitmentMap: {} });
    expect(r.status).toBe('blocked');
    expect(r.tx_hash).toBeNull();
  });

  it('refuses TRON mainnet', async () => {
    const r = await prepareTronRelease('ng-abuja', 'Phase 1', 0, 40, { ...cfg, fullNode: 'https://api.trongrid.io' });
    expect(r.status).toBe('blocked');
    expect(r.note).toMatch(/mainnet/);
  });

  it('blocks when the community has no mapped commitment', async () => {
    const r = await prepareTronRelease('unknown', 'Phase 1', 0, 40, cfg);
    expect(r.status).toBe('blocked');
  });

  it('blocks when the commitment does not exist on-chain', async () => {
    mockCount(1);
    const r = await prepareTronRelease('ng-abuja', 'Phase 1', 0, 40, cfg);
    expect(r.status).toBe('blocked');
    expect(r.note).toMatch(/does not exist/);
  });

  it('returns a dry-run (never a tx hash) when the commitment exists', async () => {
    mockCount(2);
    const r = await prepareTronRelease('ng-abuja', 'Phase 1', 0, 40, cfg);
    expect(r.status).toBe('dry-run');
    expect(r.commitment_id).toBe(1);
    expect(r.tx_hash).toBeNull();
  });

  it('blocks (not throws) when the full node is unreachable', async () => {
    globalThis.fetch = jest.fn().mockRejectedValue(new Error('network down')) as unknown as typeof fetch;
    const r = await prepareTronRelease('ng-abuja', 'Phase 1', 0, 40, cfg);
    expect(r.status).toBe('blocked');
  });
});

describe('loadTronConfig', () => {
  it('tolerates malformed commitment maps', () => {
    expect(loadTronConfig({ D3RAC_COMMITMENT_MAP: '{oops' }).commitmentMap).toEqual({});
  });
});
