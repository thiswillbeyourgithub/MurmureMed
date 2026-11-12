import { ExternalLink } from '@/components/external-link';
import { useTranslation } from '@/i18n';

// Plumed fork credits (written with Claude Code). The text lives here in
// both languages rather than in fr.json, so upstream locale files stay
// untouched and the Plumed renaming of translated strings cannot alter
// the sentences that name the original Murmure.

const PARAKEET_WEB = 'https://github.com/thiswillbeyourgithub/parakeet_web';
const ULTIMED = 'https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-onnx';
const INTERHOP = 'https://framagit.org/interhop/murmure-regex';
const LPSN = 'https://lpsn.dsmz.de/';

interface CreditsText {
    intro: [string, string, string];
    dataTitle: string;
    drugRules: [string, string];
    dictationRules: string;
    phraseList: [string, string];
    upstream: string;
}

const TEXT: Record<'fr' | 'en', CreditsText> = {
    en: {
        intro: [
            'Plumed is a fork of Murmure for French medical dictation, made by ',
            ', psychiatrist and developer. It adds the medical mode of ',
            '.',
        ],
        dataTitle: 'Bundled medical data:',
        drugRules: ['Drug and medical term fix rules: ', ' (CC BY 4.0, built on French open drug data, Etalab 2.0)'],
        dictationRules: 'Medical dictation rules: ',
        phraseList: ['Medical phrase boosting list: ', ' (AGPL v3). Bacterial names from '],
        upstream: 'Murmure is made by Al1x-ai. Please support the original project above.',
    },
    fr: {
        intro: [
            'Plumed est un fork de Murmure dédié à la dictée médicale en français, réalisé par ',
            ', psychiatre et développeur. Il reprend le mode médical de ',
            '.',
        ],
        dataTitle: 'Données médicales incluses :',
        drugRules: [
            'Règles de correction des médicaments et termes médicaux : ',
            " (CC BY 4.0, d'après les données ouvertes françaises sur le médicament, Etalab 2.0)",
        ],
        dictationRules: 'Règles de dictée médicale : ',
        phraseList: ["Liste d'amplification de phrases médicales : ", ' (AGPL v3). Noms de bactéries issus de '],
        upstream: 'Murmure est développé par Al1x-ai. Soutenez le projet original ci-dessus.',
    },
};

export const ForkCredits = () => {
    const { i18n } = useTranslation();
    const lang = i18n.language.startsWith('fr') ? 'fr' : 'en';
    const text = TEXT[lang];

    return (
        <section className="w-full space-y-3 rounded-2xl border-2 border-border/60 bg-background/50 p-5 text-left text-sm text-muted-foreground">
            <h2 className="text-center text-xl font-semibold tracking-tight text-foreground">Plumed</h2>
            <p>
                {text.intro[0]}
                <ExternalLink href={`https://olicorne.org/${lang}`} withIcon={false}>
                    Olivier Cornelis
                </ExternalLink>
                {text.intro[1]}
                <ExternalLink href={PARAKEET_WEB} withIcon={false}>
                    ParakeetWeb
                </ExternalLink>
                {text.intro[2]}
            </p>
            <div>
                <p>{text.dataTitle}</p>
                <ul className="list-disc space-y-1 pl-5">
                    <li>
                        {text.drugRules[0]}
                        <ExternalLink href={ULTIMED} withIcon={false}>
                            UltiMed
                        </ExternalLink>
                        {text.drugRules[1]}
                    </li>
                    <li>
                        {text.dictationRules}
                        <ExternalLink href={INTERHOP} withIcon={false}>
                            InterHop murmure-regex
                        </ExternalLink>{' '}
                        (AGPL v3)
                    </li>
                    <li>
                        {text.phraseList[0]}
                        <ExternalLink href={PARAKEET_WEB} withIcon={false}>
                            ParakeetWeb
                        </ExternalLink>
                        {text.phraseList[1]}
                        <ExternalLink href={LPSN} withIcon={false}>
                            LPSN
                        </ExternalLink>{' '}
                        (CC BY-SA 4.0)
                    </li>
                </ul>
            </div>
            <p className="text-xs">{text.upstream}</p>
        </section>
    );
};
