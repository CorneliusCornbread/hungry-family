import { useAuth } from './AuthContext'
import ProtectedRoute from './ProtectedRoute'
import LoginPage from './LoginPage'

function HomePage() {
  const { account, logout } = useAuth()

  return (
    <div style={{ fontFamily: 'sans-serif', maxWidth: 480, margin: '80px auto', padding: '0 1rem' }}>
      <h1>Welcome, {account.username}!</h1>
      <p style={{ marginBottom: '1.5rem', color: '#666' }}>You are logged in.</p>
      <button onClick={logout} style={{ cursor: 'pointer' }}>
        Log out
      </button>
    </div>
  )
}

export default function App() {
  return (
    <ProtectedRoute fallback={LoginPage}>
      <HomePage />
    </ProtectedRoute>
  )
}