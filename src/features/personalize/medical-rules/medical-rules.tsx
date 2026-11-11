import { useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { Button } from '@/components/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/collapsible';
import { Input } from '@/components/input';
import { ResetButton } from '@/components/reset-button';
import { SettingsUI } from '@/components/settings-ui';
import { Textarea } from '@/components/textarea';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { useDebouncedInvoke } from './hooks/use-debounced-invoke';
import { useMedicalRules } from './hooks/use-medical-rules';
import { MedicalRuleRow } from './medical-rule-row/medical-rule-row';
import { findEdit, removeEditAt, replaceEditAt, upsertEdit } from './medical-rules.helpers';
import { MedicalRuleView } from './types';

const NO_RESULTS: MedicalRuleView[] = [];

/** Advanced panel: search, change, disable or add drug and medical term rules. */
export const MedicalRules = () => {
    const { t } = useTranslation();
    const { edits, saveEdits } = useMedicalRules();
    const [query, setQuery] = useState('');
    const [sample, setSample] = useState('');
    const [newCore, setNewCore] = useState('');
    const [newReplacement, setNewReplacement] = useState('');

    const results = useDebouncedInvoke('search_medical_rules', 'query', query, NO_RESULTS);
    const preview = useDebouncedInvoke('preview_medical_fixes', 'text', sample, '', edits);

    const addRule = async () => {
        const saved = await saveEdits([
            ...edits,
            { original: null, core: newCore, replacement: newReplacement, enabled: true },
        ]);
        if (saved) {
            setNewCore('');
            setNewReplacement('');
        }
    };

    return (
        <SettingsUI.Container>
            <Collapsible>
                <div className="flex items-center gap-2 p-4">
                    <CollapsibleTrigger asChild>
                        <Button variant="ghost" size="sm" className="group" data-testid="medical-rules-toggle">
                            <ChevronDown className="transition-transform group-data-[state=open]:rotate-180" />
                            {t('Edit the drug and medical term rules')}
                        </Button>
                    </CollapsibleTrigger>
                    {edits.length > 0 && (
                        <span className="text-xs text-muted-foreground">
                            {t('{{count}} change(s)', { count: edits.length })}
                        </span>
                    )}
                    {edits.length > 0 && (
                        <div className="ml-auto">
                            <ResetButton
                                label={t('Reset all changes to the bundled rules')}
                                onClick={() => saveEdits([])}
                                data-testid="medical-rules-reset"
                            />
                        </div>
                    )}
                </div>
                <CollapsibleContent className="space-y-6 px-4 pb-4">
                    <section className="space-y-2">
                        <Typography.Title>{t('Try the rules')}</Typography.Title>
                        <Textarea
                            value={sample}
                            onChange={(e) => setSample(e.target.value)}
                            placeholder={t('Type or paste a transcription...')}
                        />
                        {preview.length > 0 && (
                            <p className="text-sm" data-testid="medical-rules-preview">
                                {preview}
                            </p>
                        )}
                    </section>

                    <section className="space-y-2">
                        <Typography.Title>{t('Find a rule')}</Typography.Title>
                        <Typography.Paragraph>
                            {t('Search by the right word, or by what the transcription wrote.')}
                        </Typography.Paragraph>
                        <Input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="rispéridone" />
                        {results.map((rule) => {
                            const edit = findEdit(edits, rule.original);
                            return (
                                <MedicalRuleRow
                                    key={rule.original}
                                    rule={edit ?? { ...rule, enabled: true }}
                                    status={edit == null ? 'bundled' : 'changed'}
                                    onSave={(changed) => saveEdits(upsertEdit(edits, changed))}
                                    onRemove={() => saveEdits(edits.filter((other) => other !== edit))}
                                />
                            );
                        })}
                    </section>

                    {edits.length > 0 && (
                        <section className="space-y-2">
                            <Typography.Title>{t('Your changes')}</Typography.Title>
                            {edits.map((edit, index) => (
                                <MedicalRuleRow
                                    key={`${edit.original ?? 'new'}-${index}`}
                                    rule={edit}
                                    status={edit.original == null ? 'added' : 'changed'}
                                    onSave={(changed) => saveEdits(replaceEditAt(edits, index, changed))}
                                    onRemove={() => saveEdits(removeEditAt(edits, index))}
                                />
                            ))}
                        </section>
                    )}

                    <section className="space-y-2">
                        <Typography.Title>{t('Add a rule')}</Typography.Title>
                        <div className="flex items-center gap-2">
                            <Input
                                value={newCore}
                                onChange={(e) => setNewCore(e.target.value)}
                                placeholder={t('Misheard text')}
                                className="font-mono text-xs"
                            />
                            <span className="text-muted-foreground">→</span>
                            <Input
                                value={newReplacement}
                                onChange={(e) => setNewReplacement(e.target.value)}
                                placeholder={t('Replacement')}
                            />
                            <Button
                                variant="secondary"
                                size="sm"
                                disabled={newCore.trim().length === 0 || newReplacement.trim().length === 0}
                                onClick={addRule}
                                data-testid="medical-rules-add"
                            >
                                {t('Add')}
                            </Button>
                        </div>
                    </section>
                </CollapsibleContent>
            </Collapsible>
        </SettingsUI.Container>
    );
};
