CREATE TABLE store_layouts(
    layout_id serial PRIMARY KEY,
    store_id int NOT NULL REFERENCES stores(store_id) ON DELETE CASCADE,
    label text NOT NULL CHECK (LENGTH(TRIM(label)) > 0),
    sort_order int NOT NULL DEFAULT 1,
    UNIQUE (store_id, label)
);

ALTER TABLE products
    ADD CONSTRAINT products_aisle_id_fk FOREIGN KEY (aisle_id) REFERENCES store_layouts(layout_id) ON DELETE SET NULL;

