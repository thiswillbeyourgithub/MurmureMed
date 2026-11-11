import { useEffect, useState } from 'react';
import { Check, Trash2 } from 'lucide-react';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { Switch } from '@/components/switch';
import { ResetButton } from '@/components/reset-button';
import { useTranslation } from '@/i18n';
import { MedicalRuleEdit } from '../types';

interface MedicalRuleRowProps {
    rule: MedicalRuleEdit;
    /** 'added' rules are deleted, 'changed' ones reverted to the bundled rule. */
    status: 'bundled' | 'changed' | 'added';
    onSave: (rule: MedicalRuleEdit) => Promise<boolean>;
    onRemove?: () => void;
}

export const MedicalRuleRow = ({ rule, status, onSave, onRemove }: MedicalRuleRowProps) => {
    const { t } = useTranslation();
    const [core, setCore] = useState(rule.core);
    const [replacement, setReplacement] = useState(rule.replacement);

    useEffect(() => {
        setCore(rule.core);
        setReplacement(rule.replacement);
    }, [rule.core, rule.replacement]);

    const isDirty = core !== rule.core || replacement !== rule.replacement;

    return (
        <div className="flex items-center gap-2 py-1" data-testid="medical-rule-row">
            <Input
                value={core}
                onChange={(e) => setCore(e.target.value)}
                aria-label={t('Misheard text')}
                className="font-mono text-xs"
            />
            <span className="text-muted-foreground">→</span>
            <Input value={replacement} onChange={(e) => setReplacement(e.target.value)} aria-label={t('Replacement')} />
            {isDirty && (
                <Button
                    variant="subtle"
                    size="icon-sm"
                    aria-label={t('Save')}
                    onClick={() => onSave({ ...rule, core, replacement })}
                >
                    <Check />
                </Button>
            )}
            <Switch
                checked={rule.enabled}
                onCheckedChange={(enabled) => onSave({ ...rule, enabled })}
                aria-label={t('Enabled')}
            />
            {status === 'changed' && onRemove != null && (
                <ResetButton label={t('Restore the bundled rule')} onClick={onRemove} />
            )}
            {status === 'added' && onRemove != null && (
                <Button variant="subtle" size="icon-sm" aria-label={t('Delete')} onClick={onRemove}>
                    <Trash2 />
                </Button>
            )}
        </div>
    );
};
