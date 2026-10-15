-- Ledger schema and a few queries to cover the SQL grammar.
/* A block comment
   over two lines */
CREATE TABLE IF NOT EXISTS accounts (
    id BIGSERIAL PRIMARY KEY,
    owner_name VARCHAR(120) NOT NULL,
    balance NUMERIC(12, 2) DEFAULT 0.00 CHECK (balance >= 0),
    is_active BOOLEAN DEFAULT TRUE,
    opened_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    tags TEXT[],
    meta JSONB
);

CREATE UNIQUE INDEX idx_accounts_owner ON accounts (lower(owner_name));

CREATE TABLE entries (
    id INTEGER GENERATED ALWAYS AS IDENTITY,
    account_id BIGINT REFERENCES accounts (id) ON DELETE CASCADE,
    amount DECIMAL(12, 2) NOT NULL,
    note TEXT,
    CONSTRAINT entries_pk PRIMARY KEY (id)
);

ALTER TABLE entries ADD COLUMN posted_on DATE;
ALTER TABLE accounts DROP COLUMN IF EXISTS meta;

INSERT INTO accounts (owner_name, balance, is_active)
VALUES ('Ada', 100.50, true), ('Grace', 0, FALSE), ('O''Brien', 12e3, NULL);

UPDATE accounts SET balance = balance - 10, is_active = false WHERE id = $1 AND owner_name LIKE 'A%';

DELETE FROM entries WHERE posted_on < CURRENT_DATE - INTERVAL '30 days';

WITH totals AS (
    SELECT a.id, a.owner_name, SUM(e.amount) AS total, COUNT(*) AS n
    FROM accounts AS a
    LEFT JOIN entries e ON e.account_id = a.id
    WHERE a.is_active IS NOT NULL
    GROUP BY a.id, a.owner_name
    HAVING COUNT(*) > 2
)
SELECT
    t.owner_name,
    t.total,
    CASE WHEN t.total > 1000 THEN 'high' WHEN t.total BETWEEN 100 AND 1000 THEN 'mid' ELSE 'low' END AS band,
    RANK() OVER (PARTITION BY t.n ORDER BY t.total DESC) AS place,
    CAST(t.total AS INTEGER) AS whole,
    t.total::text AS shown,
    COALESCE(t.n, 0) + 1 AS next_n
FROM totals t
WHERE t.owner_name IN ('Ada', 'Grace') OR t.id = ANY (ARRAY[1, 2, 3])
ORDER BY t.total DESC NULLS LAST
LIMIT 10 OFFSET 5;

SELECT DISTINCT owner_name FROM accounts
UNION ALL
SELECT note FROM entries WHERE EXISTS (SELECT 1 FROM accounts WHERE accounts.id = entries.account_id);

CREATE VIEW active_accounts AS SELECT * FROM accounts WHERE is_active;

CREATE OR REPLACE FUNCTION add_entry(acc BIGINT, amt NUMERIC) RETURNS VOID AS $$
BEGIN
    INSERT INTO entries (account_id, amount) VALUES (acc, amt);
END;
$$ LANGUAGE plpgsql;

BEGIN;
COMMIT;
DROP VIEW IF EXISTS active_accounts;
