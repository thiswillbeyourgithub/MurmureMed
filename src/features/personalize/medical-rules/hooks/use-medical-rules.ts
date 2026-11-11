import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { MedicalRuleEdit } from '../types';

/** The user's edits to the bundled medical rules, saved on every change. */
export const useMedicalRules = () => {
    const [edits, setEdits] = useState<MedicalRuleEdit[]>([]);
    const { t } = useTranslation();

    useEffect(() => {
        invoke<MedicalRuleEdit[]>('get_medical_rule_edits')
            .then(setEdits)
            .catch((error: unknown) => console.error('Failed to load medical rule edits:', error));
    }, []);

    /** Saves the whole list; the backend refuses it if one edit is invalid. */
    const saveEdits = useCallback(
        async (newEdits: MedicalRuleEdit[]): Promise<boolean> => {
            try {
                await invoke('set_medical_rule_edits', { edits: newEdits });
                setEdits(newEdits);
                return true;
            } catch (error) {
                toast.error(`${t('Invalid rule')}: ${String(error)}`);
                return false;
            }
        },
        [t]
    );

    return { edits, saveEdits };
};
