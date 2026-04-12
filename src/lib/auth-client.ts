import { createAuthClient } from 'better-auth/react';
import { fetch as tauriFetch } from '@tauri-apps/plugin-http';
import { readTextFile, writeTextFile, BaseDirectory, mkdir, remove } from '@tauri-apps/plugin-fs';

// Cookie file path
const COOKIE_FILE_PATH = 'auth-cookie.txt';

// Load cookie from file
const loadCookieFromFile = async (): Promise<string | null> => {
    try {
        const cookie = await readTextFile(COOKIE_FILE_PATH, { baseDir: BaseDirectory.AppConfig });
        return cookie.trim() || null;
    } catch {
        return null;
    }
};

// Save cookie to file
const saveCookieToFile = async (cookie: string): Promise<void> => {
    try {
        // Ensure directory exists
        await mkdir('', { baseDir: BaseDirectory.AppConfig, recursive: true }).catch(() => {
            // Directory might already exist, ignore error
        });

        // Save cookie file
        await writeTextFile(COOKIE_FILE_PATH, cookie, { baseDir: BaseDirectory.AppConfig });
        console.log('Cookie saved successfully to:', COOKIE_FILE_PATH);
    } catch (error) {
        console.error('Failed to save cookie:', error);
        // Provide more detailed error information in development environment
        if (typeof error === 'object' && error !== null) {
            console.error('Error details:', JSON.stringify(error, null, 2));
        }
    }
};

// Delete cookie file
const deleteCookieFile = async (): Promise<void> => {
    try {
        await remove(COOKIE_FILE_PATH, { baseDir: BaseDirectory.AppConfig });
        console.log('Cookie file deleted successfully');
    } catch (error) {
        console.error('Failed to delete cookie file:', error);
        // Ignore error when file doesn't exist
        if (typeof error === 'object' && error !== null) {
            console.error('Error details:', JSON.stringify(error, null, 2));
        }
    }
};

/**
 * Custom fetch implementation that handles Tauri-specific requirements and cookie management
 *
 * In Tauri v2, the HTTP plugin's fetch follows the standard fetch API, so this is simpler.
 * We still need cookie management because WKWebView on Mac doesn't include cookies automatically.
 */
export const tauriFetchImpl = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const url = typeof input === 'string' ? input : input.toString();

    // Load cookie from file
    const storedCookie = await loadCookieFromFile();

    // Build headers
    const headers = new Headers(init?.headers);

    // Add real User-Agent header
    headers.set('User-Agent', navigator.userAgent);

    if (needAppendCookies(url) && storedCookie) {
        headers.set('Cookie', storedCookie);
    }

    if (needDeleteCookies(url)) {
        await deleteCookieFile();
    }

    const response = await tauriFetch(url, {
        ...init,
        headers,
    });

    // If response contains api/auth/sign-in/, store cookie
    if (needSaveCookies(url)) {
        const setCookieHeader = response.headers.get('set-cookie');
        if (setCookieHeader) {
            // Parse cookies
            const cookies = setCookieHeader.split(',').map(c => c.trim());
            const cookieValues = cookies.map((cookie) => cookie.split(';')[0]).join('; ');

            // Check if session_token exists and is not empty
            const sessionTokenMatch = cookieValues.match(/session_token=([^;]+)/);
            if (sessionTokenMatch && sessionTokenMatch[1] && sessionTokenMatch[1].trim() !== '') {
                await saveCookieToFile(cookieValues);
            } else {
                console.log('Session token is empty or not found, skipping cookie save');
            }
        }
    }

    return response;
};

const needAppendCookies = (url: string): boolean => {
    return (
        url.includes(import.meta.env.VITE_API_BASE_URL) &&
        !url.includes('api/auth/sign-in/') &&
        !url.includes('api/auth/sign-up/')
    );
};

const needDeleteCookies = (url: string): boolean => {
    return url.includes(import.meta.env.VITE_API_BASE_URL) && url.includes('sign-out');
};

const needSaveCookies = (url: string): boolean => {
    return url.includes(import.meta.env.VITE_API_BASE_URL) && !url.includes('api/auth/sign-up/');
};

export const authClient = createAuthClient({
    baseURL: import.meta.env.VITE_API_BASE_URL,
    fetchOptions: { customFetchImpl: tauriFetchImpl },
});
