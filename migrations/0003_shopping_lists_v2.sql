CREATE TABLE standalone_products(
    standalone_product_id serial PRIMARY KEY,
    name text NOT NULL UNIQUE CHECK (LENGTH(TRIM(name)) > 0),
    is_active boolean NOT NULL DEFAULT TRUE
);

INSERT INTO standalone_products(name)
SELECT DISTINCT
    name
FROM
    products
ON CONFLICT (name)
    DO NOTHING;

CREATE TABLE store_products(
    store_product_id serial PRIMARY KEY,
    store_id int NOT NULL REFERENCES stores(store_id) ON DELETE CASCADE,
    standalone_product_id int NOT NULL REFERENCES standalone_products(standalone_product_id),
    aisle_id int REFERENCES store_layouts(layout_id) ON DELETE SET NULL,
    is_active boolean NOT NULL DEFAULT TRUE,
    UNIQUE (store_id, standalone_product_id)
);

INSERT INTO store_products(store_id, standalone_product_id, aisle_id, is_active)
SELECT
    p.store_id,
    sp.standalone_product_id,
    p.aisle_id,
    p.is_active
FROM
    products p
    JOIN standalone_products sp ON sp.name = p.name
ON CONFLICT (store_id,
    standalone_product_id)
    DO NOTHING;

CREATE TABLE store_shopping_lists(
    list_id serial PRIMARY KEY,
    store_id int NOT NULL REFERENCES stores(store_id) ON DELETE CASCADE,
    created_by int NOT NULL REFERENCES users(user_id),
    created_at timestamptz NOT NULL DEFAULT NOW(),
    closed_at timestamptz,
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'closed'))
);

CREATE UNIQUE INDEX store_shopping_lists_one_active_per_store ON store_shopping_lists(store_id)
WHERE
    status = 'active';

CREATE TABLE store_shopping_list_items(
    item_id serial PRIMARY KEY,
    list_id int NOT NULL REFERENCES store_shopping_lists(list_id) ON DELETE CASCADE,
    store_product_id int NOT NULL REFERENCES store_products(store_product_id),
    quantity int NOT NULL DEFAULT 1 CHECK (quantity > 0),
    created_at timestamptz NOT NULL DEFAULT NOW(),
    UNIQUE (list_id, store_product_id)
);

DROP TABLE IF EXISTS shopping_list_items;

DROP TABLE IF EXISTS shopping_lists;

DROP TABLE IF EXISTS products;

