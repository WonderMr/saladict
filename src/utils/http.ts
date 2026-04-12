import { fetch as pluginFetch } from '@tauri-apps/plugin-http';

export const DEFAULT_USER_AGENT = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36';
export const DEFAULT_EDGE_USER_AGENT = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.0.0 Safari/537.36 Edg/113.0.1774.42';

// V1-compatible Body class for Tauri v2 migration
export class Body {
    private _content: BodyInit;
    private _contentType?: string;

    private constructor(content: BodyInit, contentType?: string) {
        this._content = content;
        this._contentType = contentType;
    }

    static json(data: any): Body {
        return new Body(JSON.stringify(data), 'application/json');
    }

    static text(data: string): Body {
        return new Body(data);
    }

    static form(data: Record<string, string>): Body {
        return new Body(new URLSearchParams(data).toString(), 'application/x-www-form-urlencoded');
    }

    get content(): BodyInit { return this._content; }
    get contentType(): string | undefined { return this._contentType; }
}

// V1-compatible ResponseType enum
export const ResponseType = {
    JSON: 1,
    Text: 2,
    Binary: 3,
} as const;

interface V1FetchOptions {
    method?: string;
    headers?: Record<string, string>;
    body?: Body | BodyInit;
    query?: Record<string, string>;
    responseType?: number;
}

interface V1FetchResponse<T = any> {
    ok: boolean;
    status: number;
    data: T;
    headers: Record<string, string>;
}

// V1-compatible fetch wrapper over Tauri v2 plugin-http
export async function fetch<T = any>(url: string, options?: V1FetchOptions): Promise<V1FetchResponse<T>> {
    let fullUrl = url;
    if (options?.query) {
        const params = new URLSearchParams(options.query);
        const separator = url.includes('?') ? '&' : '?';
        fullUrl = `${url}${separator}${params.toString()}`;
    }

    let body: BodyInit | undefined;
    const extraHeaders: Record<string, string> = {};

    if (options?.body instanceof Body) {
        body = options.body.content;
        if (options.body.contentType) {
            extraHeaders['Content-Type'] = options.body.contentType;
        }
    } else {
        body = options?.body as BodyInit | undefined;
    }

    const response = await pluginFetch(fullUrl, {
        method: options?.method || 'GET',
        headers: { ...extraHeaders, ...options?.headers },
        body,
    });

    let data: any;
    const responseType = options?.responseType;
    if (responseType === ResponseType.Binary) {
        data = Array.from(new Uint8Array(await response.arrayBuffer()));
    } else if (responseType === ResponseType.Text) {
        data = await response.text();
    } else {
        const text = await response.text();
        try {
            data = JSON.parse(text);
        } catch {
            data = text;
        }
    }

    return {
        ok: response.ok,
        status: response.status,
        data: data as T,
        headers: Object.fromEntries(response.headers.entries()),
    };
}

export interface FetchWithUAOptions {
    method?: string;
    headers?: Record<string, string>;
    body?: BodyInit;
    query?: Record<string, string>;
    responseType?: number;
}

export async function fetchWithUA(url: string, options: FetchWithUAOptions) {
    return fetch(url, {
        ...options,
        headers: {
            ...options.headers,
            'User-Agent': DEFAULT_USER_AGENT,
        },
    });
}
