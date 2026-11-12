> [!CAUTION]
> # ⚠️ Plumed : PAS ENCORE UTILISABLE ⚠️
>
> **Ce projet n'est pas encore prêt et ne doit pas être utilisé.**
>
> Plumed vise la **dictée d'audio médical en français** (comptes rendus, observations, courriers médicaux), et non la transcription de n'importe quel type d'audio. Tant que ce bandeau est présent, considérez ce dépôt comme un travail en cours, instable et non fonctionnel.

# Plumed

Plumed est un fork de [Murmure](https://github.com/Kieirra/murmure), une application de reconnaissance vocale 100 % locale.

Son but est de porter dans une application facile à installer les fonctionnalités du **mode médical de ParakeetWeb**, déjà utilisable aujourd'hui : [http://parakeetweb.olicorne.org/?mode=med](http://parakeetweb.olicorne.org/?mode=med)

ParakeetWeb fonctionne dès maintenant, et plutôt bien, dans n'importe quel navigateur. Mais il reste limité au navigateur. Plumed doit en faire une application installable en quelques clics, pour pouvoir dicter directement dans **n'importe quelle application de l'ordinateur** (logiciel métier, traitement de texte, messagerie, etc.).

En attendant que Plumed soit prêt, utilisez ParakeetWeb.

Plumed est réalisé par [Olivier Cornelis](https://olicorne.org/fr), psychiatre et développeur.

## Confidentialité

Tout fonctionne sur votre ordinateur : l'audio et les transcriptions ne sont jamais envoyés sur internet. Seule exception : si vous connectez vous-même un serveur distant dans Extensions > Mode Prompt.

## Remerciements et licences

- **[Murmure](https://github.com/Kieirra/murmure)** par Al1x-ai : l'application dont Plumed est un fork (AGPL v3). Soutenez le projet original !
- **[ParakeetWeb](https://github.com/thiswillbeyourgithub/parakeet_web)** : le mode médical porté ici, et la liste d'amplification de phrases médicales (AGPL v3).
- **[UltiMed](https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-onnx)** : règles de correction des médicaments et termes médicaux (CC BY 4.0), construites à partir des données ouvertes françaises sur le médicament (Licence Ouverte / Etalab 2.0 : OPEN_MEDIC, RETROCEDAM, BDPM).
- **[InterHop murmure-regex](https://framagit.org/interhop/murmure-regex)** : règles de dictée médicale (unités, constantes, ponctuation dictée, modèles d'examens), proposées par défaut comme règles de formatage (AGPL v3).
- **[LPSN](https://lpsn.dsmz.de/)** (List of Prokaryotic names with Standing in Nomenclature) : noms de bactéries inclus dans la liste d'amplification de phrases, sous licence [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). Citation requise : Parte, A.C., Sardà Carbasse, J., Meier-Kolthoff, J.P., Reimer, L.C. & Göker, M. (2020). List of Prokaryotic names with Standing in Nomenclature (LPSN) moves to the DSMZ. *International Journal of Systematic and Evolutionary Microbiology* 70, 5607-5612. https://doi.org/10.1099/ijsem.0.004332
- **[NVIDIA Parakeet TDT 0.6B v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3)** : le modèle de reconnaissance vocale.

Le détail des données médicales incluses (provenance, mise à jour) est dans [src-tauri/medical-data/SOURCE.md](src-tauri/medical-data/SOURCE.md).

---

Développé avec l'aide de Claude Code.
