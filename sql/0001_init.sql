create table markets(
  id text primary key,
  question text not null,
  category text,
  created_ts timestamptz not null,
  resolution_ts timestamptz,
  status text not null
);

create table tokens(
  id text primary key,
  market_id text references markets(id),
  outcome text not null
);

create table trades(
  ts timestamptz not null,
  trade_id text primary key,
  market_id text references markets(id),
  token_id text references tokens(id),
  side text not null,
  size_shares numeric not null,
  price_cents integer not null,
  taker_maker text,
  fee_cents integer default 0,
  wallet text
);

create index trades_market_ts on trades(market_id, ts);

create table ob_snapshots(
  ts timestamptz not null,
  market_id text,
  token_id text,
  best_bid_cents integer,
  best_ask_cents integer,
  bid_sz numeric,
  ask_sz numeric
);

create table positions_daily(
  date date not null,
  market_id text,
  token_id text,
  qty numeric,
  avg_cost_cents integer,
  mtm_cents integer,
  realized_pnl_cents integer,
  unrealized_pnl_cents integer,
  primary key(date, market_id, token_id)
);
