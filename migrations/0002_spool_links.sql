CREATE TABLE spool_links (
  id INTEGER PRIMARY KEY,
  spool_id INTEGER NOT NULL REFERENCES spools(id) ON DELETE CASCADE,
  url TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0
);

INSERT INTO spool_links (spool_id, url, sort_order)
SELECT id, product_url, 0 FROM spools WHERE product_url IS NOT NULL AND product_url != '';

ALTER TABLE spools DROP COLUMN product_url;
