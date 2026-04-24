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

-- Products (depends on stores)
CREATE TABLE products(
    product_id serial PRIMARY KEY,
    name text NOT NULL UNIQUE CHECK (LENGTH(TRIM(name)) > 0),
    aisle_id int,
    store_id int NOT NULL REFERENCES stores(store_id),
    is_active boolean NOT NULL DEFAULT TRUE
);

-- Shopping lists (depends on users)
CREATE TABLE shopping_lists(
    list_id serial PRIMARY KEY,
    name text NOT NULL CHECK (LENGTH(TRIM(name)) > 0),
    created_by int NOT NULL REFERENCES users(user_id),
    created_at timestamptz NOT NULL DEFAULT NOW(),
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'draft', 'archived'))
);

-- Shopping list items (depends on shopping_lists + products)
CREATE TABLE shopping_list_items(
    item_id serial PRIMARY KEY,
    list_id int NOT NULL REFERENCES shopping_lists(list_id) ON DELETE CASCADE,
    product_id int NOT NULL REFERENCES products(product_id),
    quantity int NOT NULL CHECK (quantity > 0),
    completed boolean NOT NULL DEFAULT FALSE,
    notes text,
    UNIQUE (list_id, product_id)
);

