ALTER TABLE products DROP CONSTRAINT IF EXISTS products_name_key;
ALTER TABLE products
    ADD CONSTRAINT products_store_name_key UNIQUE (store_id, name);

CREATE TABLE standalone_products (
    standalone_product_id SERIAL PRIMARY KEY,
    name                  TEXT NOT NULL UNIQUE CHECK (LENGTH(TRIM(name)) > 0),
    is_active             BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE store_shopping_lists (
    list_id      SERIAL PRIMARY KEY,
    store_id     INT NOT NULL REFERENCES stores(store_id) ON DELETE CASCADE,
    created_by   INT NOT NULL REFERENCES users(user_id),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at    TIMESTAMPTZ,
    status       TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'closed'))
);

CREATE UNIQUE INDEX store_shopping_lists_one_active_per_store
    ON store_shopping_lists(store_id)
    WHERE status = 'active';

CREATE TABLE store_shopping_list_items (
    item_id      SERIAL PRIMARY KEY,
    list_id      INT NOT NULL REFERENCES store_shopping_lists(list_id) ON DELETE CASCADE,
    product_id   INT NOT NULL REFERENCES products(product_id),
    quantity     INT NOT NULL DEFAULT 1 CHECK (quantity > 0),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (list_id, product_id)
);

INSERT INTO standalone_products (name)
SELECT DISTINCT name
FROM products
ON CONFLICT (name) DO NOTHING;
