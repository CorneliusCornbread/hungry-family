-- Add migration script here
CREATE TYPE account_role as ENUM ('admin', 'user');

alter table public.accounts
    add role account_role default 'user' not null;
