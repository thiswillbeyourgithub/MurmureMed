# Dictée médicale (MurmureMed)

Fonctionnalités propres au fork MurmureMed, absentes de Murmure.

## Correction des noms de médicaments et des termes médicaux

Activée par défaut : **Personnaliser > Règles de formatage > Corriger les noms de médicaments et les termes médicaux**.

Environ 40 000 règles intégrées corrigent les noms de médicaments et les termes médicaux que la reconnaissance vocale entend mal, par exemple « respéridone » devient « rispéridone » et « autorino la ringologie » devient « otorhinolaryngologie ». Ce sont les mêmes règles que le mode médical de [ParakeetWeb](http://parakeetweb.olicorne.org/?mode=med), appliquées en local, avant toutes les autres règles de formatage. Provenance et licence (CC BY 4.0) : [`src-tauri/medical-data/SOURCE.md`](../../src-tauri/medical-data/SOURCE.md).

## Règles de dictée InterHop (installées par défaut)

L'association [InterHop](https://interhop.org/) maintient des règles de dictée médicale pour Murmure (unités, constantes, ponctuation dictée, modèles d'examen déclenchés par « ajoute la commande ... ») : [framagit.org/interhop/murmure-regex](https://framagit.org/interhop/murmure-regex). Par exemple « tension 120 80 » devient « TA : 120/80 » et « 3 fois par jour » devient « 3/j ».

Une nouvelle installation de MurmureMed les contient déjà dans **Personnaliser > Règles de formatage**, comme règles personnalisées ordinaires : vous pouvez les modifier, les désactiver ou les supprimer, et ajouter les vôtres. Elles s'appliquent après la correction des médicaments.

Sur une installation existante (ou après les avoir supprimées), vous pouvez les importer :

1. Télécharger [`config.murmure`](https://framagit.org/interhop/murmure-regex/-/raw/main/config.murmure).
2. Dans **Paramètres > Importer / Exporter**, importer ce fichier avec la stratégie **Fusionner**, pour garder vos propres règles (**Remplacer** les supprime). Les règles InterHop déjà présentes sont mises à jour, pas dupliquées.

En ligne de commande : `murmure import config.murmure --strategy merge`.

Attention : l'import, quelle que soit la stratégie, applique aussi les options intégrées d'InterHop (espace avant ? et !, conversion des nombres en chiffres en français, etc.). La correction des médicaments reste activée.

Rédigé avec Claude Code.
