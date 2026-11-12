import { ShieldCheck } from 'lucide-react';
import { useTranslation } from '@/i18n';

// Plumed fork (written with Claude Code): states on the home page that
// medical dictation stays on the computer. Text kept here in both languages
// so upstream locale files stay untouched. The exception is real: Prompt
// Mode can send transcripts to a remote server the user configures (behind
// its own privacy acknowledgment), so it must not be hidden.

const TEXT = {
    en: {
        title: '100% local: your audio and transcripts never leave this computer.',
        detail: 'Nothing is sent over the internet, unless you connect a remote server yourself in Extensions > Prompt Mode.',
    },
    fr: {
        title: '100 % local : votre audio et vos transcriptions ne quittent jamais cet ordinateur.',
        detail: "Rien n'est envoyé sur internet, sauf si vous connectez vous-même un serveur distant dans Extensions > Mode Prompt.",
    },
};

export const PrivacyNotice = () => {
    const { i18n } = useTranslation();
    const text = TEXT[i18n.language.startsWith('fr') ? 'fr' : 'en'];

    return (
        <div className="flex items-start gap-3 rounded-md border border-emerald-500/40 bg-emerald-500/10 p-3 text-sm">
            <ShieldCheck className="mt-0.5 h-5 w-5 shrink-0 text-emerald-400" aria-hidden="true" />
            <div className="space-y-0.5">
                <p className="font-semibold text-foreground">{text.title}</p>
                <p className="text-xs text-muted-foreground">{text.detail}</p>
            </div>
        </div>
    );
};
