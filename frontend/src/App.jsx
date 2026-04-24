import { useEffect, useMemo, useState } from 'react'
import { useAuth } from './AuthContext'
import LoginPage from './LoginPage'
import './App.css'

function generateLayout(layoutType, start, end) {
  if (layoutType === 'custom') {
    return []
  }

  if (layoutType === 'number') {
    const startNum = Number.parseInt(start, 10)
    const endNum = Number.parseInt(end, 10)
    if (Number.isNaN(startNum) || Number.isNaN(endNum) || endNum < startNum) {
      return []
    }

    return Array.from({ length: endNum - startNum + 1 }, (_, idx) => `Aisle ${startNum + idx}`)
  }

  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'
  const startLetter = String(start || 'A').toUpperCase()
  const count = Number.parseInt(end, 10) || 1
  const startIdx = alphabet.indexOf(startLetter[0])

  if (startIdx < 0 || count < 1) {
    return []
  }

  return Array.from({ length: count }, (_, idx) => alphabet[startIdx + idx])
    .filter(Boolean)
    .map((letter) => `Aisle ${letter}`)
}

async function api(path, init = {}) {
  const res = await fetch(path, {
    ...init,
    credentials: 'same-origin',
    headers: {
      'Content-Type': 'application/json',
      ...(init.headers || {}),
    },
  })

  if (!res.ok) {
    const text = await res.text()
    throw new Error(text || `Request failed (${res.status})`)
  }

  if (res.status === 204) {
    return null
  }

  return res.json()
}

function Dashboard({ onNavigate, onLogout, username }) {
  return (
    <main className="shell-page">
      <section className="shell-card">
        <h1>Dashboard</h1>
        <p className="muted">Welcome back, {username}.</p>
        <p className="muted">This is a starter dashboard shell. We can add cards/widgets here over time.</p>
        <div className="shell-actions">
          <button onClick={() => onNavigate('/store-planner')}>Open Store Planner</button>
          <button className="secondary" onClick={onLogout}>
            Log out
          </button>
        </div>
      </section>
    </main>
  )
}

function StorePlanner({ onNavigate, onLogout, username }) {
  const [stores, setStores] = useState([])
  const [products, setProducts] = useState([])
  const [selectedStoreId, setSelectedStoreId] = useState(null)
  const [pageError, setPageError] = useState('')

  const [storeForm, setStoreForm] = useState({
    storeName: '',
    storeAddress: '',
    layoutType: 'number',
    layoutStart: '1',
    layoutEnd: '12',
  })

  const [zoneLabel, setZoneLabel] = useState('')
  const [productId, setProductId] = useState('')
  const [productZone, setProductZone] = useState('')

  const [editLayoutId, setEditLayoutId] = useState(null)
  const [editLabel, setEditLabel] = useState('')
  const [editSortOrder, setEditSortOrder] = useState(1)

  const selectedStore = useMemo(
    () => stores.find((store) => store.store_id === selectedStoreId) ?? null,
    [stores, selectedStoreId],
  )

  const availableZones = selectedStore?.layouts ?? []

  async function loadStores() {
    const nextStores = await api('/api/planner/stores')
    setStores(nextStores)
    if (!selectedStoreId && nextStores.length > 0) {
      setSelectedStoreId(nextStores[0].store_id)
    }
  }

  async function loadProducts(storeId) {
    const nextProducts = await api(`/api/planner/stores/${storeId}/products`)
    setProducts(nextProducts)
    if (nextProducts.length > 0) {
      setProductId(String(nextProducts[0].product_id))
    } else {
      setProductId('')
    }
  }

  useEffect(() => {
    loadStores().catch((error) => setPageError(error.message))
  }, [])

  useEffect(() => {
    if (!selectedStoreId) return
    loadProducts(selectedStoreId).catch((error) => setPageError(error.message))
  }, [selectedStoreId])

  async function handleStoreSubmit(event) {
    event.preventDefault()
    setPageError('')

    const name = storeForm.storeName.trim()
    const address = storeForm.storeAddress.trim()
    if (!name || !address) {
      setPageError('Store name and address are required.')
      return
    }

    const store = await api('/api/planner/stores', {
      method: 'POST',
      body: JSON.stringify({ name, address }),
    })

    const starterLabels = generateLayout(storeForm.layoutType, storeForm.layoutStart, storeForm.layoutEnd)
    for (const label of starterLabels) {
      await api(`/api/planner/stores/${store.store_id}/layouts`, {
        method: 'POST',
        body: JSON.stringify({ label }),
      })
    }

    await loadStores()
    setSelectedStoreId(store.store_id)
    setStoreForm({
      storeName: '',
      storeAddress: '',
      layoutType: 'number',
      layoutStart: '1',
      layoutEnd: '12',
    })
  }

  async function handleAddZone(event) {
    event.preventDefault()
    if (!selectedStore) return

    const label = zoneLabel.trim()
    if (!label) return

    await api(`/api/planner/stores/${selectedStore.store_id}/layouts`, {
      method: 'POST',
      body: JSON.stringify({ label }),
    })

    setZoneLabel('')
    await loadStores()
  }

  async function handleSaveLayout() {
    if (!editLayoutId) return

    await api(`/api/planner/layouts/${editLayoutId}`, {
      method: 'PATCH',
      body: JSON.stringify({ label: editLabel, sort_order: Number(editSortOrder) || 1 }),
    })

    setEditLayoutId(null)
    setEditLabel('')
    setEditSortOrder(1)
    await loadStores()
  }

  async function handleDeleteLayout(layoutId) {
    await api(`/api/planner/layouts/${layoutId}`, { method: 'DELETE' })
    await loadStores()
    await loadProducts(selectedStoreId)
  }

  async function handleAssignProduct(event) {
    event.preventDefault()
    if (!selectedStoreId || !productId) return

    await api(`/api/planner/stores/${selectedStoreId}/product-layout`, {
      method: 'PATCH',
      body: JSON.stringify({
        product_id: Number(productId),
        layout_id: productZone ? Number(productZone) : null,
      }),
    })

    await loadProducts(selectedStoreId)
  }

  return (
    <main className="planner-page">
      <header className="planner-header">
        <div>
          <h1>Store + Layout Planner</h1>
          <p className="lead">
            Signed in as <strong>{username}</strong>. Layouts and product associations are saved to the
            database.
          </p>
        </div>
        <div className="header-actions">
          <button className="secondary" onClick={() => onNavigate('/dashboard')}>
            Back to Dashboard
          </button>
          <button className="secondary logout" onClick={onLogout}>
            Log out
          </button>
        </div>
      </header>

      <p className="error">{pageError}</p>

      <div className="planner-grid">
        <section className="panel">
          <h2>1) Add Store</h2>
          <form onSubmit={handleStoreSubmit}>
            <label>
              Store name
              <input
                value={storeForm.storeName}
                onChange={(event) => setStoreForm((prev) => ({ ...prev, storeName: event.target.value }))}
                placeholder="e.g. Woodman's"
                required
              />
            </label>

            <label>
              Address
              <input
                value={storeForm.storeAddress}
                onChange={(event) => setStoreForm((prev) => ({ ...prev, storeAddress: event.target.value }))}
                placeholder="e.g. 123 Main St"
                required
              />
            </label>

            <label>
              Starter layout type
              <select
                value={storeForm.layoutType}
                onChange={(event) => setStoreForm((prev) => ({ ...prev, layoutType: event.target.value }))}
              >
                <option value="number">Aisles by number (1, 2, 3...)</option>
                <option value="letter">Aisles by letter (A, B, C...)</option>
                <option value="custom">No starter labels</option>
              </select>
            </label>

            <div className="row">
              <label>
                Start
                <input
                  value={storeForm.layoutStart}
                  onChange={(event) => setStoreForm((prev) => ({ ...prev, layoutStart: event.target.value }))}
                />
              </label>

              <label>
                End / Count
                <input
                  value={storeForm.layoutEnd}
                  onChange={(event) => setStoreForm((prev) => ({ ...prev, layoutEnd: event.target.value }))}
                />
              </label>
            </div>

            <button type="submit">Create store</button>
          </form>

          <ul className="list">
            {stores.length === 0 && <li className="muted">No stores yet.</li>}
            {stores.map((store) => (
              <li
                key={store.store_id}
                className={`list-item ${selectedStoreId === store.store_id ? 'active-store' : ''}`}
              >
                <strong>{store.name}</strong>
                <div className="muted">{store.address}</div>
                <button className="secondary" type="button" onClick={() => setSelectedStoreId(store.store_id)}>
                  {selectedStoreId === store.store_id ? 'Selected' : 'Manage layout'}
                </button>
              </li>
            ))}
          </ul>
        </section>

        <section className="panel">
          <h2>2) Modify Layout</h2>
          <p className="muted">
            {selectedStore
              ? `Editing ${selectedStore.name} (${selectedStore.layouts.length} locations).`
              : 'Choose a store to edit its layout.'}
          </p>

          <form onSubmit={handleAddZone}>
            <label>
              Add location label
              <input
                value={zoneLabel}
                onChange={(event) => setZoneLabel(event.target.value)}
                placeholder="e.g. Produce or B12"
              />
            </label>
            <button type="submit" className="secondary">
              Add location
            </button>
          </form>

          <ul className="list">
            {!selectedStore && <li className="muted">Select a store first.</li>}
            {selectedStore && selectedStore.layouts.length === 0 && <li className="muted">No locations yet.</li>}
            {selectedStore?.layouts.map((layout) => (
              <li key={layout.layout_id} className="list-item">
                {editLayoutId === layout.layout_id ? (
                  <>
                    <div className="row">
                      <input value={editLabel} onChange={(event) => setEditLabel(event.target.value)} />
                      <input
                        type="number"
                        value={editSortOrder}
                        onChange={(event) => setEditSortOrder(event.target.value)}
                      />
                    </div>
                    <div className="row">
                      <button type="button" onClick={handleSaveLayout}>
                        Save
                      </button>
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => {
                          setEditLayoutId(null)
                          setEditLabel('')
                        }}
                      >
                        Cancel
                      </button>
                    </div>
                  </>
                ) : (
                  <>
                    <strong>{layout.label}</strong>
                    <div className="muted">Sort #{layout.sort_order}</div>
                    <div className="row">
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => {
                          setEditLayoutId(layout.layout_id)
                          setEditLabel(layout.label)
                          setEditSortOrder(layout.sort_order)
                        }}
                      >
                        Edit
                      </button>
                      <button type="button" className="secondary" onClick={() => handleDeleteLayout(layout.layout_id)}>
                        Delete
                      </button>
                    </div>
                  </>
                )}
              </li>
            ))}
          </ul>
        </section>

        <section className="panel">
          <h2>3) Associate Existing Products</h2>
          <p className="muted">Products are loaded from the database for the selected store.</p>

          <form onSubmit={handleAssignProduct}>
            <label>
              Product
              <select value={productId} onChange={(event) => setProductId(event.target.value)}>
                {products.length === 0 && <option value="">No products for this store</option>}
                {products.map((product) => (
                  <option key={product.product_id} value={product.product_id}>
                    {product.name}
                  </option>
                ))}
              </select>
            </label>

            <label>
              Store location
              <select value={productZone} onChange={(event) => setProductZone(event.target.value)}>
                <option value="">Unassigned</option>
                {availableZones.map((zone) => (
                  <option key={zone.layout_id} value={zone.layout_id}>
                    {zone.label}
                  </option>
                ))}
              </select>
            </label>

            <button type="submit">Save product location</button>
          </form>

          <ul className="list">
            {products.length === 0 && <li className="muted">No active products found for this store.</li>}
            {products.map((product) => {
              const layout = availableZones.find((zone) => zone.layout_id === product.aisle_id)
              return (
                <li key={product.product_id} className="list-item">
                  <strong>{product.name}</strong>
                  <div className="muted">Location: {layout?.label ?? 'Unassigned'}</div>
                </li>
              )
            })}
          </ul>
        </section>
      </div>
    </main>
  )
}

function navigateTo(path, setPathname) {
  window.history.pushState({}, '', path)
  setPathname(path)
}

function replaceTo(path, setPathname) {
  window.history.replaceState({}, '', path)
  setPathname(path)
}

export default function App() {
  const { account, logout } = useAuth()
  const [pathname, setPathname] = useState(window.location.pathname)

  useEffect(() => {
    const onPopState = () => setPathname(window.location.pathname)
    window.addEventListener('popstate', onPopState)
    return () => window.removeEventListener('popstate', onPopState)
  }, [])

  useEffect(() => {
    if (account === null) return

    if (pathname === '/') {
      if (account === false) {
        replaceTo('/login', setPathname)
      } else {
        replaceTo('/dashboard', setPathname)
      }
    }
  }, [account, pathname])

  if (account === null) {
    return (
      <div className="loading-shell">
        <span className="spinner" aria-label="Loading session" />
      </div>
    )
  }

  if (pathname === '/login') {
    if (account !== false) {
      replaceTo('/dashboard', setPathname)
      return null
    }
    return <LoginPage />
  }

  if (pathname === '/store-planner') {
    if (account === false) {
      replaceTo('/login', setPathname)
      return null
    }

    return (
      <StorePlanner
        username={account.username}
        onLogout={async () => {
          await logout()
          replaceTo('/login', setPathname)
        }}
        onNavigate={(path) => navigateTo(path, setPathname)}
      />
    )
  }

  if (pathname === '/dashboard') {
    if (account === false) {
      replaceTo('/login', setPathname)
      return null
    }

    return (
      <Dashboard
        username={account.username}
        onLogout={async () => {
          await logout()
          replaceTo('/login', setPathname)
        }}
        onNavigate={(path) => navigateTo(path, setPathname)}
      />
    )
  }

  replaceTo('/', setPathname)
  return null
}