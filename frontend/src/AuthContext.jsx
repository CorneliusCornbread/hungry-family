import { useEffect, useState } from 'react'
import { AuthContext } from './useAuth'

export function AuthProvider({ children }) {
    // null = unknown (loading), false = logged out, object = logged in
    const [account, setAccount] = useState(null)

    // On mount, check if we already have a valid session.
    useEffect(() => {
        fetch('/api/auth/me', { credentials: 'same-origin' })
            .then(r => r.ok ? r.json() : null)
            .then(data => setAccount(data ?? false))
            .catch(() => setAccount(false))
    }, [])

    async function login(username, password) {
        const res = await fetch('/api/auth/login', {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password }),
        })
        const data = await res.json()
        if (!res.ok) throw new Error(data.error ?? 'Login failed')
        // Re-fetch the account so we have full account data.
        const me = await fetch('/api/auth/me', { credentials: 'same-origin' }).then(r => r.json())
        setAccount(me)
    }

    async function logout() {
        await fetch('/api/auth/logout', { method: 'POST', credentials: 'same-origin' })
        setAccount(false)
    }

    return (
        <AuthContext.Provider value={{ account, login, logout }}>
            {children}
        </AuthContext.Provider>
    )
}
