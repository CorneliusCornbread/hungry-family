import { useAuth } from './useAuth'

/**
 * Wrap any route element with this to require authentication.
 * Shows a loading spinner while the session check is in-flight,
 * then renders the login page or the children accordingly.
 */
export default function ProtectedRoute({ children, fallback }) {
    const { account } = useAuth()

    // Still checking session with the server
    if (account === null) {
        return (
            <div style={{ display: 'grid', placeItems: 'center', minHeight: '100vh' }}>
                <span className="spinner" aria-label="Loading…" />
            </div>
        )
    }

    if (account === false) {
        return fallback ?? null
    }

    return children
}
