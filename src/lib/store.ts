import { writable, type Writable } from 'svelte/store';
import { initializeDatabase } from './db.js';

// Check if we're in browser environment
const browser = typeof window !== 'undefined';

// User interface
interface User {
    id: string | null;
    username: string | null;
    isLoggedIn: boolean;
}

// User store - stores user ID and username
export const user: Writable<User> = writable({
    id: null,
    username: null,
    isLoggedIn: false
});

// Database store - stores SQLite instance
export const db: Writable<any> = writable(null);

// Session timeout store
export const sessionTimeout: Writable<Date | null> = writable(null);

// Initialize stores
export async function initializeStores(): Promise<void> {
    try {
        // Initialize database
        const database = await initializeDatabase();
        db.set(database);
        
        // Load last username from localStorage
        if (browser) {
            const lastUsername = localStorage.getItem('anpos_last_username') || '';
            user.update(current => ({
                ...current,
                username: lastUsername
            }));
        }
        
        console.log('Stores initialized successfully');
    } catch (error) {
        console.error('Failed to initialize stores:', error);
        throw error;
    }
}

// Login function
export function login(userId: string, username: string): void {
    user.set({
        id: userId,
        username: username,
        isLoggedIn: true
    });
    
    // Save username to localStorage
    if (browser) {
        localStorage.setItem('anpos_last_username', username);
    }
    
    // Reset session timeout
    resetSessionTimeout();
}

// Logout function
export function logout(): void {
    user.set({
        id: null,
        username: null,
        isLoggedIn: false
    });
    
    // Clear session timeout
    clearSessionTimeout();
    
    // Note: We keep the username in localStorage for convenience
}

// Session timeout management
let timeoutId: NodeJS.Timeout | null = null;

export function resetSessionTimeout(): void {
    clearSessionTimeout();
    
    // Set 15-minute timeout
    timeoutId = setTimeout(() => {
        console.log('Session timeout - logging out');
        logout();
        // Redirect to login page
        if (browser) {
            window.location.href = '/';
        }
    }, 15 * 60 * 1000); // 15 minutes
    
    sessionTimeout.set(new Date(Date.now() + 15 * 60 * 1000));
}

export function clearSessionTimeout(): void {
    if (timeoutId) {
        clearTimeout(timeoutId);
        timeoutId = null;
    }
    sessionTimeout.set(null);
}

// Setup activity listeners for session timeout reset
export function setupActivityListeners(): (() => void) | undefined {
    if (!browser) return;
    
    const resetTimeout = () => {
        const currentUser = get(user);
        if (currentUser && currentUser.isLoggedIn) {
            resetSessionTimeout();
        }
    };
    
    // Listen for user activity
    window.addEventListener('click', resetTimeout);
    window.addEventListener('keydown', resetTimeout);
    window.addEventListener('mousemove', resetTimeout);
    
    // Cleanup function
    return () => {
        window.removeEventListener('click', resetTimeout);
        window.removeEventListener('keydown', resetTimeout);
        window.removeEventListener('mousemove', resetTimeout);
    };
}

// Helper function to get store value
export function get<T>(store: Writable<T>): T {
    let value: T;
    store.subscribe((val: T) => value = val)();
    return value!;
} 