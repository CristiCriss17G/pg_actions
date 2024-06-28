#!/bin/sh

# Initialize the destinations string
DESTINATIONS=""

# loop through the tags and generate a --destination argument for each one
while read TAG; do
    DESTINATION="--destination=${HARBOR_REPO}/${HARBOR_PROJ}/${SERVICE_NAME}:${TAG}"
    DESTINATIONS="$DESTINATIONS $DESTINATION"
done <$TAGS

# pass the destinations arguments to /kaniko/executor
/kaniko/executor --context "$PWD" \
    --dockerfile "$PWD/Dockerfile" \
    $DESTINATIONS \
    --build-arg RUST_VERSION \
    --build-arg ALPINE_VERSION \
    --cleanup
