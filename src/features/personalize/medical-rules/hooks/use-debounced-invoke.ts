import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

const DEBOUNCE_MS = 250;

/**
 * Calls the Tauri `command` with `{ [argName]: input }` once `input` stops
 * changing, and returns its latest result. `refreshKey` forces a new call
 * (e.g. after the rules changed). Empty input gives `emptyResult`.
 */
export const useDebouncedInvoke = <T>(
    command: string,
    argName: string,
    input: string,
    emptyResult: T,
    refreshKey: unknown = null
): T => {
    const [result, setResult] = useState<T>(emptyResult);

    useEffect(() => {
        if (input.trim().length === 0) {
            setResult(emptyResult);
            return;
        }
        let cancelled = false;
        const timer = setTimeout(() => {
            invoke<T>(command, { [argName]: input })
                .then((value) => {
                    if (!cancelled) {
                        setResult(value);
                    }
                })
                .catch((error: unknown) => console.error(`${command} failed:`, error));
        }, DEBOUNCE_MS);
        return () => {
            cancelled = true;
            clearTimeout(timer);
        };
    }, [command, argName, input, refreshKey]);

    return result;
};
