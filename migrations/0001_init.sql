CREATE TABLE brands (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  tare_weight_g REAL NOT NULL,
  notes TEXT
);

CREATE TABLE materials (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  notes TEXT
);

CREATE TABLE colours (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  hex TEXT
);

CREATE TABLE spools (
  id INTEGER PRIMARY KEY,
  brand_id INTEGER NOT NULL REFERENCES brands(id),
  material_id INTEGER NOT NULL REFERENCES materials(id),
  colour_id INTEGER NOT NULL REFERENCES colours(id),
  nominal_weight_g REAL NOT NULL DEFAULT 1000,
  tare_override_g REAL,
  product_url TEXT,
  notes TEXT,
  purchase_date TEXT
);

CREATE TABLE spool_images (
  id INTEGER PRIMARY KEY,
  spool_id INTEGER NOT NULL REFERENCES spools(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE weight_log (
  id INTEGER PRIMARY KEY,
  spool_id INTEGER NOT NULL REFERENCES spools(id) ON DELETE CASCADE,
  gross_weight_g REAL NOT NULL,
  measured_at TEXT NOT NULL
);
