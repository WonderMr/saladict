const ANALYTICS_SCRIPT_ID = 'saladict-umami-analytics';

export function initAnalytics() {
    const websiteId = import.meta.env.VITE_UMAMI_DATA_WEBSITE_ID;
    const scriptUrl = import.meta.env.VITE_UMAMI_SCRIPT_URL;
    if (!websiteId || !scriptUrl) return;
    if (document.getElementById(ANALYTICS_SCRIPT_ID)) return;
    const script = document.createElement('script');
    script.id = ANALYTICS_SCRIPT_ID;
    script.defer = true;
    script.src = scriptUrl;
    script.setAttribute('data-website-id', websiteId);
    document.head.appendChild(script);
}
