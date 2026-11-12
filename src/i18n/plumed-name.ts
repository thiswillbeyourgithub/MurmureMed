import type { PostProcessorModule } from 'i18next';

// MurmureMed fork (written with Claude Code): shows the fork's name in every
// translated string without editing the upstream components and locale keys,
// which would conflict on every upstream merge.

// These ask users to support the original Murmure project: keep its name.
const KEEP_ORIGINAL_NAME = new Set([
    'Support Murmure',
    'Help Murmure Grow',
    'Murmure is free and open-source. Your support helps keep it alive and improving.',
    'Free, 1 click. The simplest way to support Murmure.',
    'Support Murmure with a one-time or recurring tip.',
]);

const UPSTREAM_NAME = /\bMurmure\b/g;

export const renameToMurmureMed = (text: string): string => text.replace(UPSTREAM_NAME, 'MurmureMed');

export const murmureMedName: PostProcessorModule = {
    type: 'postProcessor',
    name: 'murmuremed-name',
    process: (value: string, key: string | string[]) => {
        const keys = Array.isArray(key) ? key : [key];
        if (keys.some((k) => KEEP_ORIGINAL_NAME.has(k))) {
            return value;
        }
        return renameToMurmureMed(value);
    },
};
