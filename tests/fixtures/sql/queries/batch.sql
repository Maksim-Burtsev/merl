-- #472: a script whose statements end without a `;`, T-SQL batches split by GO: where a
-- common table expression's scope ends is not known.
WITH coupons AS (
  SELECT id FROM coupons
  --             ^ d: schema/001_tables.sql:17
)
SELECT id FROM coupons
GO
SELECT id FROM coupons
--             ^ d: picker queries/batch.sql:3, schema/001_tables.sql:17
