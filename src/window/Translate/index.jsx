import { readDir, BaseDirectory, readTextFile, exists } from '@tauri-apps/plugin-fs';
import { DragDropContext, Draggable, Droppable } from 'react-beautiful-dnd';
import { getCurrentWindow, currentMonitor } from '@tauri-apps/api/window';
import { appConfigDir, join } from '@tauri-apps/api/path';
import { convertFileSrc } from '@tauri-apps/api/core';
import { Spacer, Button } from '@nextui-org/react';
import { AiFillCloseCircle } from 'react-icons/ai';
import React, { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { BsPinFill } from 'react-icons/bs';

import LanguageArea from './components/LanguageArea';
import SourceArea from './components/SourceArea';
import TargetArea from './components/TargetArea';
import { osType } from '../../utils/env';
import { useConfig } from '../../hooks';
import { store } from '../../utils/store';
import { info, warn } from '@tauri-apps/plugin-log';
import { default_translate_service_list } from '../../services/translate/constants';

const appWindow = getCurrentWindow();

let blurTimeout = null;
let resizeTimeout = null;
let moveTimeout = null;

const listenBlur = () => {
    return listen('tauri://blur', () => {
        if (appWindow.label === 'translate') {
            if (blurTimeout) {
                clearTimeout(blurTimeout);
            }
            info('Blur');
            // 100ms后关闭窗口，因为在 windows 下拖动窗口时会先切换成 blur 再立即切换成 focus
            // 如果直接关闭将导致窗口无法拖动
            blurTimeout = setTimeout(async () => {
                info('Confirm Blur');
                await appWindow.close();
            }, 100);
        }
    });
};

let unlisten = listenBlur();
// 取消 blur 监听
const unlistenBlur = () => {
    unlisten.then((f) => {
        f();
    });
};

// 监听 focus 事件取消 blurTimeout 时间之内的关闭窗口
void listen('tauri://focus', () => {
    info('Focus');
    if (blurTimeout) {
        info('Cancel Close');
        clearTimeout(blurTimeout);
    }
});
// 监听 move 事件取消 blurTimeout 时间之内的关闭窗口
void listen('tauri://move', () => {
    info('Move');
    if (blurTimeout) {
        info('Cancel Close');
        clearTimeout(blurTimeout);
    }
});

export default function Translate() {
    const [closeOnBlur] = useConfig('translate_close_on_blur', true);
    const [alwaysOnTop] = useConfig('translate_always_on_top', false);
    const [windowPosition] = useConfig('translate_window_position', 'mouse');
    const [rememberWindowSize] = useConfig('translate_remember_window_size', false);
    const [translateServiceInstanceList, setTranslateServiceInstanceList] = useConfig(
        'translate_service_list',
        default_translate_service_list
    );
    const [recognizeServiceInstanceList] = useConfig('recognize_service_list', ['system', 'tesseract']);
    const [ttsServiceInstanceList] = useConfig('tts_service_list', ['lingva_tts']);
    const [collectionServiceInstanceList] = useConfig('collection_service_list', []);
    const [hideLanguage] = useConfig('hide_language', false);
    const [pined, setPined] = useState(false);
    const [pluginList, setPluginList] = useState(null);
    const [serviceInstanceConfigMap, setServiceInstanceConfigMap] = useState(null);
    const reorder = (list, startIndex, endIndex) => {
        const result = Array.from(list);
        const [removed] = result.splice(startIndex, 1);
        result.splice(endIndex, 0, removed);
        return result;
    };

    const onDragEnd = async (result) => {
        if (!result.destination) return;
        const items = reorder(translateServiceInstanceList, result.source.index, result.destination.index);
        setTranslateServiceInstanceList(items);
    };
    // 是否自动关闭窗口
    useEffect(() => {
        if (closeOnBlur !== null && !closeOnBlur) {
            unlistenBlur();
        }
    }, [closeOnBlur]);
    // 是否默认置顶
    useEffect(() => {
        if (alwaysOnTop !== null && alwaysOnTop) {
            appWindow.setAlwaysOnTop(true);
            unlistenBlur();
            setPined(true);
        }
    }, [alwaysOnTop]);
    // 保存窗口位置
    useEffect(() => {
        if (windowPosition !== null && windowPosition === 'pre_state') {
            const savePosition = async () => {
                if (appWindow.label !== 'translate') return;
                // currentMonitor() can resolve to null when the OS can't
                // determine the monitor (common on Wayland compositors and
                // during teardown). Fall back to a scaleFactor of 1.0 so the
                // save still succeeds — a slightly off HiDPI coordinate is
                // far better than crashing the handler with
                // `Cannot read properties of null`.
                const monitor = await currentMonitor();
                const factor = monitor ? monitor.scaleFactor : 1.0;
                const position = (await appWindow.outerPosition()).toLogical(factor);
                await store.set('translate_window_position_x', parseInt(position.x));
                await store.set('translate_window_position_y', parseInt(position.y));
                await store.save();
            };
            // Save only on user-initiated movement or close.
            //
            // Two feedback loops had to be killed to make this stop drifting
            // a few pixels right on every reopen:
            //
            // 1. An earlier version saved right after this effect bound —
            //    that captured the WM's post-layout nudge and fed it back in.
            //    Dropped that on-mount save; Rust's mouse-cursor fallback
            //    handles the first-open case instead.
            //
            // 2. Rust's set_position(saved_x, saved_y) when the window is
            //    first built also fires tauri://move, and the WM's tiny
            //    frame-offset adjustment fires another one, both of which
            //    land while this listener is already bound. That read-back
            //    would persist (saved + offset) and accumulate the same
            //    drift. Ignore move events for the initial 500ms — by that
            //    time the compositor has settled and any later move is a
            //    real user drag.
            const mountTime = Date.now();
            const MOVE_IGNORE_MS = 500;
            const unlistenMove = listen('tauri://move', async () => {
                if (Date.now() - mountTime < MOVE_IGNORE_MS) return;
                if (moveTimeout) {
                    clearTimeout(moveTimeout);
                }
                moveTimeout = setTimeout(() => {
                    savePosition().catch((e) =>
                        warn(`Translate: savePosition failed: ${e}`)
                    );
                }, 100);
            });
            // close-on-blur has a 100ms grace period that races with the
            // 100ms move-debounce — if the user drags the window and clicks
            // away immediately the save can lose the race. Defer the actual
            // close so savePosition() is guaranteed to complete before the
            // window goes away (otherwise the async await runs detached and
            // the tauri runtime tears the webview down first).
            const unlistenClose = appWindow.onCloseRequested(async (event) => {
                event.preventDefault();
                if (moveTimeout) {
                    clearTimeout(moveTimeout);
                    moveTimeout = null;
                }
                try {
                    await savePosition();
                } finally {
                    await appWindow.destroy();
                }
            });
            return () => {
                // Clear any pending debounced save so it doesn't fire after
                // the listeners are unbound (e.g. when the user switches
                // translate_window_position away from pre_state mid-session).
                if (moveTimeout) {
                    clearTimeout(moveTimeout);
                    moveTimeout = null;
                }
                unlistenMove.then((f) => {
                    f();
                });
                unlistenClose.then((f) => {
                    f();
                });
            };
        }
    }, [windowPosition]);
    // 保存窗口大小
    useEffect(() => {
        if (rememberWindowSize !== null && rememberWindowSize) {
            const saveSize = async () => {
                if (appWindow.label !== 'translate') return;
                // currentMonitor() can resolve to null on Wayland / teardown;
                // fall back to scaleFactor=1.0 rather than crashing the handler.
                const monitor = await currentMonitor();
                const factor = monitor ? monitor.scaleFactor : 1.0;
                const size = (await appWindow.outerSize()).toLogical(factor);
                await store.set('translate_window_height', parseInt(size.height));
                await store.set('translate_window_width', parseInt(size.width));
                await store.save();
            };
            const unlistenResize = listen('tauri://resize', async () => {
                if (resizeTimeout) {
                    clearTimeout(resizeTimeout);
                }
                resizeTimeout = setTimeout(() => {
                    saveSize().catch((e) => warn(`Translate: saveSize failed: ${e}`));
                }, 100);
            });
            return () => {
                if (resizeTimeout) {
                    clearTimeout(resizeTimeout);
                    resizeTimeout = null;
                }
                unlistenResize.then((f) => {
                    f();
                });
            };
        }
    }, [rememberWindowSize]);

    const loadPluginList = async () => {
        const serviceTypeList = ['translate', 'tts', 'recognize', 'collection'];
        let temp = {};
        for (const serviceType of serviceTypeList) {
            temp[serviceType] = {};
            if (await exists(`plugins/${serviceType}`, { baseDir: BaseDirectory.AppConfig })) {
                const plugins = await readDir(`plugins/${serviceType}`, { baseDir: BaseDirectory.AppConfig });
                for (const plugin of plugins) {
                    const infoStr = await readTextFile(`plugins/${serviceType}/${plugin.name}/info.json`, {
                        baseDir: BaseDirectory.AppConfig,
                    });
                    let pluginInfo = JSON.parse(infoStr);
                    if ('icon' in pluginInfo) {
                        const appConfigDirPath = await appConfigDir();
                        const iconPath = await join(
                            appConfigDirPath,
                            `/plugins/${serviceType}/${plugin.name}/${pluginInfo.icon}`
                        );
                        pluginInfo.icon = convertFileSrc(iconPath);
                    }
                    temp[serviceType][plugin.name] = pluginInfo;
                }
            }
        }
        setPluginList({ ...temp });
    };

    useEffect(() => {
        loadPluginList();
        if (!unlisten) {
            unlisten = listen('reload_plugin_list', loadPluginList);
        }
    }, []);

    const loadServiceInstanceConfigMap = async () => {
        const config = {};
        for (const serviceInstanceKey of translateServiceInstanceList) {
            config[serviceInstanceKey] = (await store.get(serviceInstanceKey)) ?? {};
        }
        for (const serviceInstanceKey of recognizeServiceInstanceList) {
            config[serviceInstanceKey] = (await store.get(serviceInstanceKey)) ?? {};
        }
        for (const serviceInstanceKey of ttsServiceInstanceList) {
            config[serviceInstanceKey] = (await store.get(serviceInstanceKey)) ?? {};
        }
        for (const serviceInstanceKey of collectionServiceInstanceList) {
            config[serviceInstanceKey] = (await store.get(serviceInstanceKey)) ?? {};
        }
        setServiceInstanceConfigMap({ ...config });
    };
    useEffect(() => {
        if (
            translateServiceInstanceList !== null &&
            recognizeServiceInstanceList !== null &&
            ttsServiceInstanceList !== null &&
            collectionServiceInstanceList !== null
        ) {
            loadServiceInstanceConfigMap();
        }
    }, [
        translateServiceInstanceList,
        recognizeServiceInstanceList,
        ttsServiceInstanceList,
        collectionServiceInstanceList,
    ]);

    return (
        pluginList && (
            <div
                className={`bg-background h-screen w-screen ${
                    osType === 'Linux' && 'rounded-[10px] border-1 border-default-100'
                }`}
            >
                <div
                    className='fixed top-[5px] left-[5px] right-[5px] h-[30px]'
                    data-tauri-drag-region='true'
                />
                <div className={`h-[35px] w-full flex ${osType === 'Darwin' ? 'justify-end' : 'justify-between'}`}>
                    <Button
                        isIconOnly
                        size='sm'
                        variant='flat'
                        disableAnimation
                        className='my-auto bg-transparent'
                        onPress={() => {
                            if (pined) {
                                if (closeOnBlur) {
                                    unlisten = listenBlur();
                                }
                                appWindow.setAlwaysOnTop(false);
                            } else {
                                unlistenBlur();
                                appWindow.setAlwaysOnTop(true);
                            }
                            setPined(!pined);
                        }}
                    >
                        <BsPinFill className={`text-[20px] ${pined ? 'text-primary' : 'text-default-400'}`} />
                    </Button>
                    <Button
                        isIconOnly
                        size='sm'
                        variant='flat'
                        disableAnimation
                        className={`my-auto ${osType === 'Darwin' && 'hidden'} bg-transparent`}
                        onPress={() => {
                            void appWindow.close();
                        }}
                    >
                        <AiFillCloseCircle className='text-[20px] text-default-400' />
                    </Button>
                </div>
                <div className={`${osType === 'Linux' ? 'h-[calc(100vh-37px)]' : 'h-[calc(100vh-35px)]'} px-[8px]`}>
                    <div className='h-full overflow-y-auto scrollbar-hide'>
                        <div>
                            {serviceInstanceConfigMap !== null && (
                                <SourceArea
                                    pluginList={pluginList}
                                    serviceInstanceConfigMap={serviceInstanceConfigMap}
                                />
                            )}
                        </div>
                        <div className={`${hideLanguage && 'hidden'}`}>
                            <LanguageArea />
                            <Spacer y={2} />
                        </div>
                        <DragDropContext onDragEnd={onDragEnd}>
                            <Droppable
                                droppableId='droppable'
                                direction='vertical'
                            >
                                {(provided) => (
                                    <div
                                        ref={provided.innerRef}
                                        {...provided.droppableProps}
                                    >
                                        {(() => {
                                            const firstEnabledIndex =
                                                translateServiceInstanceList?.findIndex((serviceInstanceKey) => {
                                                    const config = serviceInstanceConfigMap?.[serviceInstanceKey] ?? {};
                                                    return config['enable'] ?? true;
                                                }) ?? -1;

                                            return (
                                                translateServiceInstanceList !== null &&
                                                serviceInstanceConfigMap !== null &&
                                                translateServiceInstanceList.map((serviceInstanceKey, index) => {
                                                    const config = serviceInstanceConfigMap[serviceInstanceKey] ?? {};
                                                    const enable = config['enable'] ?? true;

                                                    return enable ? (
                                                        <Draggable
                                                            key={serviceInstanceKey}
                                                            draggableId={serviceInstanceKey}
                                                            index={index}
                                                        >
                                                            {(provided) => (
                                                                <div
                                                                    ref={provided.innerRef}
                                                                    {...provided.draggableProps}
                                                                >
                                                                    <TargetArea
                                                                        {...provided.dragHandleProps}
                                                                        index={index}
                                                                        name={serviceInstanceKey}
                                                                        translateServiceInstanceList={
                                                                            translateServiceInstanceList
                                                                        }
                                                                        pluginList={pluginList}
                                                                        serviceInstanceConfigMap={
                                                                            serviceInstanceConfigMap
                                                                        }
                                                                        isFirstEnabledTransService={
                                                                            index === firstEnabledIndex
                                                                        }
                                                                    />
                                                                    <Spacer y={2} />
                                                                </div>
                                                            )}
                                                        </Draggable>
                                                    ) : (
                                                        <></>
                                                    );
                                                })
                                            );
                                        })()}
                                    </div>
                                )}
                            </Droppable>
                        </DragDropContext>
                    </div>
                </div>
            </div>
        )
    );
}
