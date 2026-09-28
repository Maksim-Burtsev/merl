-- The shop of #307: every d case carries its answer in a comment under it.
WITH heavy AS (
  SELECT * FROM basket_lines WHERE weigh(weight) > 30
  --            ^ d: schema/001_tables.sql:29
  --                               ^ d: schema/002_functions.sql:5
), cheap AS (
  SELECT * FROM coupons WHERE rate < 1
  --            ^ d: schema/001_tables.sql:17
  --                          ^ d: none
)
SELECT h.tariff_id, c.describe, discount(c.rate)
--                    ^ d: none
--                              ^ d: schema/002_functions.sql:1
FROM heavy h JOIN cheap c ON c.id = h.coupon_id;
--   ^ d: queries/report.sql:2
--                ^ d: queries/report.sql:6

WITH couriers AS (
  SELECT id, name FROM couriers WHERE channel = 'post'
  --                   ^ d: picker queries/report.sql:18, schema/001_tables.sql:23; want schema/001_tables.sql:23 (#472)
)
SELECT name FROM couriers;
--               ^ d: picker queries/report.sql:18, schema/001_tables.sql:23; want queries/report.sql:18 (#472)

SELECT t.rate, g.total FROM shop.tariffs t JOIN gross g ON g.id = t.id;
--                          ^ d: picker schema/001_tables.sql:3, schema/001_tables.sql:9, schema/001_tables.sql:11; want schema/001_tables.sql:3 (#471)
--                               ^ d: schema/001_tables.sql:11
--                                              ^ d: schema/002_functions.sql:24

SELECT nextval('shop.parcel_seq')::grams, 'post'::channel;
--                   ^ d: schema/001_tables.sql:9
--                                   ^ d: schema/001_tables.sql:7
--                                                ^ d: schema/001_tables.sql:8

CALL restock(2);
--   ^ d: schema/002_functions.sql:9
GRANT SELECT ON couriers TO clerk;
--                          ^ d: schema/001_tables.sql:4
ALTER USER auditor SET search_path = shop;
--         ^ d: schema/001_tables.sql:5
DROP TRIGGER basket_touched ON basket_lines;
--           ^ d: schema/002_functions.sql:21
REINDEX INDEX couriers_name_idx;
--            ^ d: schema/001_tables.sql:35
SELECT * FROM archived_baskets UNION ALL SELECT * FROM legacy_parcels;
--            ^ d: none; want queries/archive.sql:2 (#436)
--                                                     ^ d: none; want queries/legacy.sql:2 (#436)
SELECT * FROM parcel_log, parcels;
--            ^ d: schema/003_log.mysql:1
--                        ^ d: none

ALTER EXTENSION pgcrypto UPDATE;
--              ^ d: schema/001_tables.sql:2
ALTER DATABASE depot SET timezone = 'UTC';
--             ^ d: schema/001_tables.sql:6
SELECT * FROM coupon_rates;
--            ^ d: schema/002_functions.sql:27
