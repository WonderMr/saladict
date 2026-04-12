import { type, arch as archFn, version } from '@tauri-apps/plugin-os';
import { getVersion, getName } from '@tauri-apps/api/app';

export let osType = '';
export let arch = '';
export let osVersion = '';
export let appVersion = '';
export let appName = '';

// Map v2 plugin-os type() values to v1 values used throughout the codebase
const osTypeMap = { 'linux': 'Linux', 'macos': 'Darwin', 'windows': 'Windows_NT' };

export async function initEnv() {
    const rawType = type();
    osType = osTypeMap[rawType] || rawType;
    arch = archFn();
    osVersion = version();
    appVersion = await getVersion();
    appName = await getName();
}
