import { type, arch as archFn, version } from '@tauri-apps/plugin-os';
import { getVersion, getName } from '@tauri-apps/api/app';

export let osType = '';
export let arch = '';
export let osVersion = '';
export let appVersion = '';
export let appName = '';

export async function initEnv() {
    osType = type();
    arch = archFn();
    osVersion = version();
    appVersion = await getVersion();
    appName = await getName();
}
