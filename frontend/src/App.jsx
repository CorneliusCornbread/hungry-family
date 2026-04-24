import { useMemo, useState } from 'react'
import { useAuth } from './AuthContext'
import ProtectedRoute from './ProtectedRoute'
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

function HomePage() {
  const { account, logout } = useAuth()

  const [stores, setStores] = useState([])
  const [selectedStoreId, setSelectedStoreId] = useState(null)
  const [storeError, setStoreError] = useState('')

  const [storeForm, setStoreForm] = useState({
    storeName: '',
    storeAddress: '',
    layoutType: 'number',
    layoutStart: '1',
    layoutEnd: '12',
  })

  const [zoneLabel, setZoneLabel] = useState('')
  const [productName, setProductName] = useState('')
  const [productZone, setProductZone] = useState('')

  const selectedStore = useMemo(
    () => stores.find((store) => store.id === selectedStoreId) ?? null,
    [stores, selectedStoreId],
  )

  const availableZones = selectedStore?.zones ?? []

  function handleStoreSubmit(event) {
    event.preventDefault()
    const name = storeForm.storeName.trim()

    if (!name) {
      setStoreError('Store name is required.')
      return
    }

    const zones = generateLayout(storeForm.layoutType, storeForm.layoutStart, storeForm.layoutEnd)

    const nextStore = {
      id: crypto.randomUUID(),
      name,
      address: storeForm.storeAddress.trim(),
      layoutType: storeForm.layoutType,
      zones,
      products: [],
    }

    setStores((prev) => [...prev, nextStore])
    setSelectedStoreId(nextStore.id)
    setStoreError('')
    setStoreForm({
      storeName: '',
      storeAddress: '',
      layoutType: 'number',
      layoutStart: '1',
      layoutEnd: '12',
    })
    setZoneLabel('')
    setProductName('')
    setProductZone('')
  }

  function handleAddZone(event) {
    event.preventDefault()
    if (!selectedStore) return

    const label = zoneLabel.trim()
    if (!label) return

    const duplicate = selectedStore.zones.some((zone) => zone.toLowerCase() === label.toLowerCase())
    if (duplicate) return

    setStores((prev) =>
      prev.map((store) =>
        store.id === selectedStore.id ? { ...store, zones: [...store.zones, label] } : store,
      ),
    )
    setZoneLabel('')
  }

  function handleAddProduct(event) {
    event.preventDefault()
    if (!selectedStore) return

    const trimmedName = productName.trim()
    const zone = productZone || selectedStore.zones[0] || ''
    if (!trimmedName || !zone) return

    const nextProduct = { name: trimmedName, zone }

    setStores((prev) =>
      prev.map((store) =>
        store.id === selectedStore.id
          ? { ...store, products: [...store.products, nextProduct] }
          : store,
      ),
    )
    setProductName('')
  }

  return (
    <main className="planner-page">
      <header className="planner-header">
        <div>
          <h1>Store + Layout Planner</h1>
          <p className="lead">
            Signed in as <strong>{account.username}</strong>. Add stores, define aisle labels, and map
            products to locations.
          </p>
        </div>
        <button className="secondary logout" onClick={logout}>
          Log out
        </button>
      </header>

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
              Address (optional)
              <input
                value={storeForm.storeAddress}
                onChange={(event) => setStoreForm((prev) => ({ ...prev, storeAddress: event.target.value }))}
                placeholder="e.g. 123 Main St"
              />
            </label>

            <label>
              Layout type
              <select
                value={storeForm.layoutType}
                onChange={(event) => setStoreForm((prev) => ({ ...prev, layoutType: event.target.value }))}
              >
                <option value="number">Aisles by number (1, 2, 3...)</option>
                <option value="letter">Aisles by letter (A, B, C...)</option>
                <option value="custom">Custom labels</option>
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

          <p className="error" aria-live="polite">
            {storeError}
          </p>

          <ul className="list">
            {stores.length === 0 && <li className="muted">No stores yet.</li>}
            {stores.map((store) => (
              <li key={store.id} className={`list-item ${selectedStoreId === store.id ? 'active-store' : ''}`}>
                <strong>{store.name}</strong>
                <div className="muted">{store.address || 'No address set'}</div>
                <div className="tag">{store.layoutType} layout</div>
                <button className="secondary" type="button" onClick={() => setSelectedStoreId(store.id)}>
                  {selectedStoreId === store.id ? 'Selected' : 'Manage layout'}
                </button>
              </li>
            ))}
          </ul>
        </section>

        <section className="panel">
          <h2>2) Edit Layout</h2>
          <p className="muted">
            {selectedStore
              ? `Editing ${selectedStore.name} layout (${selectedStore.zones.length} locations).`
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
            {selectedStore && selectedStore.zones.length === 0 && <li className="muted">No locations yet.</li>}
            {selectedStore?.zones.map((zone, idx) => (
              <li key={`${zone}-${idx}`} className="list-item">
                <strong>{zone}</strong> <span className="muted">#{idx + 1}</span>
              </li>
            ))}
          </ul>
        </section>

        <section className="panel">
          <h2>3) Associate Products</h2>
          <p className="muted">Connected to the Vite React frontend, ready for API wiring next.</p>

          <form onSubmit={handleAddProduct}>
            <label>
              Product name
              <input
                value={productName}
                onChange={(event) => setProductName(event.target.value)}
                placeholder="e.g. Pasta sauce"
                required
              />
            </label>

            <label>
              Store location
              <select value={productZone} onChange={(event) => setProductZone(event.target.value)}>
                {availableZones.length === 0 && <option value="">No locations available</option>}
                {availableZones.map((zone) => (
                  <option key={zone} value={zone}>
                    {zone}
                  </option>
                ))}
              </select>
            </label>

            <button type="submit">Assign product to location</button>
          </form>

          <ul className="list">
            {!selectedStore && <li className="muted">Select a store first.</li>}
            {selectedStore && selectedStore.products.length === 0 && (
              <li className="muted">No products assigned yet.</li>
            )}
            {selectedStore?.products.map((product, idx) => (
              <li key={`${product.name}-${product.zone}-${idx}`} className="list-item">
                <strong>{product.name}</strong>
                <div className="muted">Location: {product.zone}</div>
              </li>
            ))}
          </ul>
        </section>
      </div>
    </main>
  )
}

export default function App() {
  return (
    <ProtectedRoute fallback={LoginPage}>
      <HomePage />
    </ProtectedRoute>
  )
}
