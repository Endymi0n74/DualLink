# AGENTS.md — KUMO

## Canary
- Commence chaque réponse par « Stephane, ». Sans exception, y compris pour les réponses courtes et les résumés d'outils.
- Si le contexte te semble dégradé ou la session très longue, dis-le en tête de réponse et propose une nouvelle session.
- Avant une nouvelle session, écris `HANDOFF.md` : objectif, fait, reste à faire, fichiers clés, questions ouvertes. Lis-le en premier dans la session suivante.

## Style de réponse
- Pour tout texte destiné à un humain (commentaire, message de commit, réponse), utilise le moins de mots possible. Chaque mot compte. Va droit au but. Moins est plus.
- Pas de superlatifs ni de flatterie. Ne dis pas que j'ai absolument raison. Donne la vérité brute.
- Demande une précision seulement si le blocage ne peut pas être levé par une hypothèse raisonnable. Énonce alors l'hypothèse en une ligne et avance. Une seule question par réponse au maximum.

## Code : lisibilité
- Évite les nombres et chaînes magiques. Extrais les valeurs répétées ou porteuses de sens dans des `const` ou des `enum` descriptifs. Garde inline les valeurs uniques et évidentes. Une valeur issue d'une spécification (ex. HTTP 200 OK) devient toujours une constante.
- Réduis l'indentation. Évite l'anti-pattern de la flèche : utilise les retours anticipés (`return`, `continue`).
- Noms de fonctions : moins de 30 caractères.
- Utilise des `enum` plutôt que des booléens en paramètres de fonction.
- Aère le code : une ligne vide entre les blocs logiques.
- Ajoute un commentaire court qui explique *ce que* fait le bloc et *pourquoi*. Donne un exemple quand c'est utile. Pour expliquer un système complet, propose un schéma en ASCII.
- Ne touche pas au code non lié à la fonctionnalité. Ne commente pas un bloc que tu n'as ni créé ni modifié. Minimise le nombre de lignes modifiées.

## Code : architecture
- Traite tout changement de visibilité comme un changement de conception majeur. Garde tous les champs et fonctions privés, sauf si l'accès externe est strictement requis. Demande une approbation explicite avant de passer un membre de `private` à `internal` ou `public`.
- Programme par niveaux d'abstraction. La mécanique bas niveau (I/O matérielles brutes, parsing de secteurs, flux socket directs) est encapsulée dans une couche driver/abstraction dédiée. Le reste de l'application n'utilise que des API haut niveau, exprimées en concepts métier.
- Respecte strictement la hiérarchie des couches : chaque couche ne communique qu'avec la couche immédiatement en dessous. Ne perfore jamais les couches. Les contrôleurs et composants UI n'appellent jamais directement une requête base de données, un driver matériel ou un client réseau bas niveau : ils passent par la couche de service ou d'abstraction intermédiaire.

## Syntaxe
- Utilise toujours `{}`, même pour un `if` d'une seule ligne.

## Commits
Respecte ces 7 règles :
1. Sépare le sujet du corps par une seule ligne vide.
2. Limite le sujet à 50 caractères (72 est la limite absolue).
3. Mets la première lettre du sujet en majuscule.
4. Ne termine pas le sujet par un point.
5. Utilise l'impératif dans le sujet (ex. « Corrige le bug », « Ajoute la fonction », pas « Corrigé » ni « Ajoute »). Test : le sujet doit compléter la phrase « Si appliqué, ce commit va [sujet] ».
6. Coupe le corps à 72 caractères par ligne à la main.
7. Le corps explique le *quoi* et le *pourquoi*, pas le *comment*. Le code montre déjà le comment. Le message donne le contexte et le raisonnement.

## Bugs : test d'abord
- Si la demande est une correction de bug, n'écris pas le correctif tout de suite. Écris d'abord le test. Observe-le échouer. Écris ensuite le correctif. Observe le test passer.
