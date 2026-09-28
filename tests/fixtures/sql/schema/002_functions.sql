CREATE OR REPLACE FUNCTION discount(total numeric) RETURNS numeric AS $$
  SELECT least(total, 100) - 1
$$ LANGUAGE sql;

CREATE FUNCTION weigh(g grams) RETURNS integer AS $$
  SELECT g / 1000
$$ LANGUAGE sql;

CREATE PROCEDURE restock(n integer) LANGUAGE sql AS $$
  INSERT INTO couriers (name) VALUES ('post')
  --          ^ d: schema/001_tables.sql:23
$$;

CREATE FUNCTION touch_basket() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  NEW.weight := weigh(NEW.weight);
  RETURN NEW;
END
$$;

CREATE TRIGGER basket_touched BEFORE INSERT ON basket_lines
  FOR EACH ROW EXECUTE FUNCTION touch_basket();

CREATE MATERIALIZED VIEW gross AS
  SELECT t.id, discount(t.rate) AS total FROM shop.tariffs t;

CREATE VIEW coupon_rates AS
  SELECT id, rate FROM coupons;

-- A schema-qualified name declares the name, never the schema (#471).
CREATE TABLE public.receipts (id integer);
CREATE VIEW "shop"."daily_rates" AS
  SELECT id, rate FROM coupons;
