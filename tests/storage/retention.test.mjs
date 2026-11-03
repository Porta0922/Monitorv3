import { PGlite } from '@electric-sql/pglite';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

// PGlite runs PostgreSQL in WASM; it does not emulate TimescaleDB.
const source = readFileSync(new URL('../../server/migrations/0001_server_schema.sql', import.meta.url), 'utf8');
const script = readFileSync(new URL('../../scripts/maintenance/retention.sql', import.meta.url), 'utf8');
const device = '00000000-0000-0000-0000-000000000001';
async function fixture() {
  const db = new PGlite();
  for (const table of ['devices', 'node_resource_metrics', 'input_activity_metrics']) {
    const ddl = source.match(new RegExp(`CREATE TABLE IF NOT EXISTS ${table} \\([\\s\\S]*?\\n            \\)`));
    assert.ok(ddl, `Missing canonical schema: ${table}`);
    await db.exec(ddl[0]);
  }
  await db.exec(readFileSync(new URL('../../server/migrations/0002_daily_metrics.sql', import.meta.url), 'utf8'));
  for (const name of ['0010_input_quality.sql','0011_session_events.sql','0017_session_daily.sql']) await db.exec(readFileSync(new URL('../../server/migrations/'+name,import.meta.url),'utf8'));
  await db.exec(`INSERT INTO devices(device_id,hostname,last_seen) VALUES ('${device}','test',now());`);
  return db;
}
function sql(raw = 14, summary = 365, batch = 5000) {
  // The single-session engine executes the advisory-lock acquired branch.
  // The psql wrapper also has a real PostgreSQL integration test in CI.
  return script.replace(/SELECT pg_try_advisory_xact_lock[^\n]*\n/, '')
    .replace(/\\else[\s\S]*?\\endif\n/, '')
    .replace(/^\\.*$/gm, '')
    .replaceAll(":'raw_days'", `'${raw}'`)
    .replaceAll(":'summary_days'", `'${summary}'`)
    .replaceAll(":'batch_size'", `'${batch}'`);
}
async function seed(db, days, cpu = 20) {
  await db.exec(`INSERT INTO node_resource_metrics(device_id,timestamp,cpu_percent,memory_percent,memory_used_mb)
    VALUES ('${device}',date_trunc('day',now())-interval '${days} days',${cpu},40,1024)`);
}
async function count(db, table) {
  return Number((await db.query(`SELECT count(*) AS n FROM ${table}`)).rows[0].n);
}

test('bounded compaction, weighted totals, retry and late arrival', async () => {
  const db = await fixture();
  try {
    await seed(db, 20, 20); await seed(db, 20, 80); await seed(db, 1);
    await db.exec(sql(14, 365, 1));
    assert.equal(await count(db, 'node_resource_metrics'), 2);
    await db.exec(sql(14, 365, 1));
    await db.exec(sql(14, 365, 1));
    let row = (await db.query('SELECT * FROM resource_metrics_daily')).rows[0];
    assert.equal(Number(row.samples), 2);
    assert.equal(row.cpu_sum / Number(row.samples), 50);
    assert.equal(row.cpu_max, 80);
    await seed(db, 20, 50);
    await db.exec(sql());
    row = (await db.query('SELECT * FROM resource_metrics_daily')).rows[0];
    assert.equal(Number(row.samples), 3);
    assert.equal(row.cpu_sum, 150);
    assert.equal(await count(db, 'node_resource_metrics'), 1);
  } finally { await db.close(); }
});

test('UTC boundary, expired summaries and input totals', async () => {
  const db = await fixture();
  try {
    await seed(db, 14); await seed(db, 400); await seed(db, 20);
    await db.exec(`INSERT INTO input_activity_metrics(id,device_id,timestamp,active_seconds,idle_seconds,keys_count)
      VALUES ('${device}','${device}',now()-interval '20 days',40,20,10)`);
    await db.exec(sql());
    assert.equal(await count(db, 'node_resource_metrics'), 1);
    assert.equal(await count(db, 'resource_metrics_daily'), 1);
    const input = (await db.query('SELECT * FROM input_metrics_daily')).rows[0];
    assert.equal(Number(input.active_seconds), 40);
    assert.equal(Number(input.idle_seconds), 20);
    await db.exec("UPDATE resource_metrics_daily SET day = current_date - 400");
    await db.exec(sql());
    assert.equal(await count(db, 'resource_metrics_daily'), 0);
  } finally { await db.close(); }
});

test('aggregation failure rolls back deletion and success marker', async () => {
  const db = await fixture();
  try {
    await db.exec(sql());
    await seed(db, 20);
    await db.exec('ALTER TABLE resource_metrics_daily ADD CONSTRAINT injected_failure CHECK (samples < 0)');
    await assert.rejects(db.exec(sql()));
    await db.exec('ROLLBACK');
    assert.equal(await count(db, 'node_resource_metrics'), 1);
    assert.equal(await count(db, 'resource_metrics_daily'), 0);
    const row = (await db.query('SELECT * FROM maintenance_status')).rows[0];
    assert.equal(Number(row.resource_rows_compacted), 0);
  } finally { await db.close(); }
});

test('invalid retention fails without deleting samples', async () => {
  const db = await fixture();
  try {
    await seed(db, 20);
    await assert.rejects(db.exec(sql(14, 7)));
    await db.exec('ROLLBACK');
    assert.equal(await count(db, 'node_resource_metrics'), 1);
  } finally { await db.close(); }
});

const dailyQuery = readFileSync(new URL('../../server/src/domains/dashboard/daily_metrics.sql', import.meta.url), 'utf8');
async function daily(db, from = '2000-01-01', to = '2099-12-31') {
  return (await db.query(dailyQuery, [device, from, to])).rows;
}

test('daily query preserves weighted averages through partial compaction and late arrivals', async () => {
  const db = await fixture();
  try {
    await seed(db, 20, 20);
    for (let i = 0; i < 9; i++) await seed(db, 20, 80);
    let [row] = await daily(db);
    assert.equal(Number(row.resource_samples), 10);
    assert.equal(row.cpu_average, 74);
    assert.equal(row.source, 'detail');
    await db.exec(sql(14, 365, 1));
    [row] = await daily(db);
    assert.equal(row.source, 'mixed');
    assert.equal(row.cpu_average, 74);
    assert.equal(Number(row.resource_samples), 10);
    await db.exec(sql());
    [row] = await daily(db);
    assert.equal(row.source, 'summary');
    assert.equal(row.cpu_average, 74);
    await seed(db, 20, 50);
    [row] = await daily(db);
    assert.equal(row.source, 'mixed');
    assert.equal(Number(row.resource_samples), 11);
    assert.ok(Math.abs(row.cpu_average - 790 / 11) < 1e-9);
    await db.exec(sql()); await db.exec(sql());
    [row] = await daily(db);
    assert.equal(Number(row.resource_samples), 11);
    assert.ok(Math.abs(row.cpu_average - 790 / 11) < 1e-9);
  } finally { await db.close(); }
});

test('daily query respects inclusive UTC dates and device boundaries', async () => {
  const db = await fixture();
  try {
    const other = '00000000-0000-0000-0000-000000000002';
    await db.exec(`INSERT INTO devices(device_id,hostname,last_seen) VALUES ('${other}','other',now());
      INSERT INTO node_resource_metrics(device_id,timestamp,cpu_percent) VALUES
      ('${device}','2025-02-01T00:00:00Z',0),
      ('${device}','2025-02-02T00:00:00Z',99),
      ('${other}','2025-02-01T00:00:00Z',100);`);
    const rows = await daily(db, '2025-02-01', '2025-02-01');
    assert.equal(rows.length, 1);
    assert.equal(Number(rows[0].resource_samples), 1);
    assert.equal(rows[0].cpu_average, 0);
    assert.equal(rows[0].active_seconds, null);
    assert.equal((await daily(db, '2025-01-01', '2025-01-31')).length, 0);
  } finally { await db.close(); }
});

test('legacy machine input remains stored but is not mixed with person-session totals', async () => {
 const db=await fixture();
 try {
  await db.exec(`INSERT INTO input_activity_metrics(id,device_id,timestamp,active_seconds,idle_seconds,keys_count) VALUES ('${device}','${device}',now()-interval '20 days',0,30,0)`);
  assert.deepEqual(await daily(db),[]);
  await db.exec(sql());
  assert.deepEqual(await daily(db),[]);
  assert.equal(await count(db,'input_metrics_daily'),1);
 } finally {await db.close();}
});
