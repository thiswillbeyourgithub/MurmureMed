# Dictée médicale (MurmureMed)

Fonctionnalités propres au fork MurmureMed, absentes de Murmure.

## Correction des noms de médicaments et des termes médicaux

Activée par défaut : **Personnaliser > Règles de formatage > Corriger les noms de médicaments et les termes médicaux**.

Environ 40 000 règles intégrées corrigent les noms de médicaments et les termes médicaux que la reconnaissance vocale entend mal, par exemple « respéridone » devient « rispéridone » et « autorino la ringologie » devient « otorhinolaryngologie ». Ce sont les mêmes règles que le mode médical de [ParakeetWeb](http://parakeetweb.olicorne.org/?mode=med), appliquées en local, avant toutes les autres règles de formatage. Provenance et licence (CC BY 4.0) : [`src-tauri/medical-data/SOURCE.md`](../../src-tauri/medical-data/SOURCE.md).

## Règles de dictée InterHop (import manuel)

L'association [InterHop](https://interhop.org/) maintient des règles de dictée médicale pour Murmure (ponctuation dictée, unités, abréviations, etc.) : [framagit.org/interhop/murmure-regex](https://framagit.org/interhop/murmure-regex). Ce dépôt n'ayant pas de licence, MurmureMed ne les intègre pas : il faut les importer soi-même.

1. Télécharger [`config.murmure`](https://framagit.org/interhop/murmure-regex/-/raw/main/config.murmure).
2. Dans **Paramètres > Importer / Exporter**, importer ce fichier avec la stratégie **Fusionner**, pour garder vos propres règles personnalisées (**Remplacer** les supprime).

En ligne de commande : `murmure import config.murmure --strategy merge`.

Attention : l'import, quelle que soit la stratégie, applique aussi les options intégrées d'InterHop (espace avant ? et !, conversion des nombres en chiffres en français, etc.). La correction des médicaments reste activée. Les règles importées apparaissent dans les règles personnalisées et s'appliquent après la correction des médicaments.

Rédigé avec Claude Code.
