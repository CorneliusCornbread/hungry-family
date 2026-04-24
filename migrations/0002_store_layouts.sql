CREATE TABLE store_layouts (
    layout_id    SERIAL PRIMARY KEY,
    store_id     INT NOT NULL REFERENCES stores(store_id) ON DELETE CASCADE,
    label        TEXT NOT NULL CHECK (LENGTH(TRIM(label)) > 0),
    sort_order   INT NOT NULL DEFAULT 1,
    UNIQUE (store_id, label)
);

ALTER TABLE products
    ADD CONSTRAINT products_aisle_id_fk
    FOREIGN KEY (aisle_id) REFERENCES store_layouts(layout_id)
    ON DELETE SET NULL;
