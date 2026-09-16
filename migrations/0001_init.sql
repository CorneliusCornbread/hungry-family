-- Stores first (no dependencies)
CREATE TABLE stores(
    store_id serial PRIMARY KEY,
    name text NOT NULL,
    address text NOT NULL
);

-- Users must come before accounts
CREATE TABLE users(
    user_id serial PRIMARY KEY,
    firstname text NOT NULL CHECK (LENGTH(TRIM(firstname)) > 0),
    lastname text NOT NULL CHECK (LENGTH(TRIM(lastname)) > 0),
    email text NOT NULL UNIQUE CHECK (LENGTH(TRIM(email)) > 0)
);

-- Auth accounts (depends on users)
CREATE TABLE accounts(
    account_id serial PRIMARY KEY,
    user_id int NOT NULL REFERENCES users(user_id),
    username text NOT NULL UNIQUE CHECK (LENGTH(TRIM(username)) > 0),
    password_hash text NOT NULL
);

-- Sessions (depends on accounts)
CREATE TABLE sessions(
    token text PRIMARY KEY,
    account_id int NOT NULL REFERENCES accounts(account_id) ON DELETE CASCADE,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT NOW()
);

-- Aisle/section layouts per store (depends on stores)
CREATE TABLE store_layouts(
    layout_id serial PRIMARY KEY,
    store_id int NOT NULL REFERENCES stores(store_id) ON DELETE CASCADE,
    label text NOT NULL CHECK (LENGTH(TRIM(label)) > 0),
    sort_order int NOT NULL DEFAULT 1,
    UNIQUE (store_id, label)
);

-- Global product catalog (name-deduped across stores)
CREATE TABLE standalone_products(
    standalone_product_id serial PRIMARY KEY,
    name text NOT NULL UNIQUE CHECK (LENGTH(TRIM(name)) > 0),
    is_active boolean NOT NULL DEFAULT TRUE
);

-- Per-store product entries with aisle assignment
CREATE TABLE store_products(
    store_product_id serial PRIMARY KEY,
    store_id int NOT NULL REFERENCES stores(store_id) ON DELETE CASCADE,
    standalone_product_id int NOT NULL REFERENCES standalone_products(standalone_product_id),
    aisle_id int REFERENCES store_layouts(layout_id) ON DELETE SET NULL,
    is_active boolean NOT NULL DEFAULT TRUE,
    UNIQUE (store_id, standalone_product_id)
);

-- One active list per store at a time
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

-- Items on a list with quantity
CREATE TABLE store_shopping_list_items(
    item_id serial PRIMARY KEY,
    list_id int NOT NULL REFERENCES store_shopping_lists(list_id) ON DELETE CASCADE,
    store_product_id int NOT NULL REFERENCES store_products(store_product_id),
    quantity int NOT NULL DEFAULT 1 CHECK (quantity > 0),
    created_at timestamptz NOT NULL DEFAULT NOW(),
    UNIQUE (list_id, store_product_id)
);
