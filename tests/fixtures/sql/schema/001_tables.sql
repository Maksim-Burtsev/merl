-- The shop of #307: every d case carries its answer in a comment under it.
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE SCHEMA shop;
CREATE ROLE clerk;
CREATE USER auditor IN ROLE clerk;
CREATE DATABASE depot;
CREATE DOMAIN grams AS integer CHECK (VALUE >= 0);
CREATE TYPE channel AS ENUM ('post', 'courier');
CREATE SEQUENCE shop.parcel_seq;

CREATE TABLE shop.tariffs (
  id integer PRIMARY KEY,
  rate numeric NOT NULL,
  describe text
);

create table if not exists "coupons" (
  id integer primary key,
  rate numeric not null,
  describe text
);

CREATE UNLOGGED TABLE couriers (
  id integer DEFAULT nextval('shop.parcel_seq'),
  name text,
  channel channel
);

CREATE TEMP TABLE basket_lines (
  tariff_id integer REFERENCES shop.tariffs (id),
  coupon_id integer REFERENCES coupons (id),
  weight grams
);

CREATE UNIQUE INDEX couriers_name_idx ON couriers (name);

/*
The first draft, kept for the record:
CREATE TABLE parcels (
  id integer
);
*/
