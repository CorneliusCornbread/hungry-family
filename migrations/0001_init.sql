CREATE DATABASE hungry_family;
USE hungry_family;

-- Stores first (no dependencies)
CREATE TABLE stores (
    store_id   SERIAL PRIMARY KEY,
    name       TEXT NOT NULL,
    address    TEXT NOT NULL
);

-- Users
CREATE TABLE users (
    user_id      SERIAL PRIMARY KEY,
    firstname    TEXT NOT NULL CHECK (LENGTH(TRIM(firstname)) > 0),
    lastname     TEXT NOT NULL CHECK (LENGTH(TRIM(lastname)) > 0),
    email        TEXT NOT NULL UNIQUE CHECK (LENGTH(TRIM(email)) > 0)
);

-- Products (depends on stores)
CREATE TABLE products (
    product_id  SERIAL PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE CHECK (LENGTH(TRIM(name)) > 0),
    aisle_id    INT,
    store_id    INT NOT NULL REFERENCES stores(store_id),
    is_active   BOOLEAN NOT NULL DEFAULT TRUE
);

-- Shopping lists (depends on users)
CREATE TABLE shopping_lists (
    list_id     SERIAL PRIMARY KEY,
    name        TEXT NOT NULL CHECK (LENGTH(TRIM(name)) > 0),
    created_by  INT NOT NULL REFERENCES users(user_id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status      TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'draft', 'archived'))
);

-- Shopping list items (depends on shopping_lists + products)
CREATE TABLE shopping_list_items (
    item_id     SERIAL PRIMARY KEY,
    list_id     INT NOT NULL REFERENCES shopping_lists(list_id) ON DELETE CASCADE,
    product_id  INT NOT NULL REFERENCES products(product_id),
    quantity    INT NOT NULL CHECK (quantity > 0),
    completed   BOOLEAN NOT NULL DEFAULT FALSE,
    notes       TEXT,
    UNIQUE (list_id, product_id)
);
