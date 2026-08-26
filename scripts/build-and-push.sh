#!/usr/bin/env bash
# Build the casts image for the deployed architecture and push it to ECR.
#
# Usage:
#   scripts/build-and-push.sh            # build and push
#   scripts/build-and-push.sh --deploy   # ...then register a task definition
#                                        #    revision and roll the service
#   scripts/build-and-push.sh --allow-dirty
#
# The image tag is the short commit sha, so an image can always be traced back
# to the source that produced it. A dirty tree is refused by default: tagging
# uncommitted work with a commit's sha produces an artifact nobody can
# reproduce, and ECR deduplicates by content, so the bad tag can end up
# pointing at an unrelated image.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
REPO_ROOT="$(pwd)"

ACCOUNT=690591551001
REGION=us-east-1
REGISTRY="${ACCOUNT}.dkr.ecr.${REGION}.amazonaws.com"
IMAGE="${REGISTRY}/casts"
CLUSTER=commit-store
SERVICE=casts
TASKDEF="${REPO_ROOT}/deploy/ecs/task-definition.json"
# The build context is the PARENT directory: casts depends on commit-store by
# the relative path ../../commit-store, so both checkouts must be in scope.
CONTEXT="$(dirname "$REPO_ROOT")"
DOCKERFILE="$(basename "$REPO_ROOT")/deploy/casts/Dockerfile"

DEPLOY=0
ALLOW_DIRTY=0
for arg in "$@"; do
  case "$arg" in
    --deploy) DEPLOY=1 ;;
    --allow-dirty) ALLOW_DIRTY=1 ;;
    *) echo "unknown option: $arg" >&2; exit 64 ;;
  esac
done

TAG="$(git rev-parse --short HEAD)"
if ! git diff --quiet HEAD -- ':!*.md'; then
  if [ "$ALLOW_DIRTY" -eq 1 ]; then
    TAG="${TAG}-dirty"
    echo "WARNING: tree is dirty; tagging ${TAG}. Do not deploy this to production." >&2
  else
    echo "Uncommitted source changes; the tag would not describe the image." >&2
    echo "Commit first, or pass --allow-dirty to build ${TAG}-dirty." >&2
    git status --short -- ':!*.md' >&2
    exit 1
  fi
fi

echo "==> logging in to ECR"
aws ecr get-login-password --region "$REGION" \
  | docker login --username AWS --password-stdin "$REGISTRY"

# linux/arm64 is not optional: the task definition sets cpuArchitecture ARM64,
# and a mismatched image fails at task start rather than at build time.
echo "==> building ${IMAGE}:${TAG} (linux/arm64)"
docker build --platform linux/arm64 -f "$CONTEXT/$DOCKERFILE" -t "${IMAGE}:${TAG}" "$CONTEXT"

echo "==> pushing"
docker push "${IMAGE}:${TAG}"

if [ "$DEPLOY" -eq 0 ]; then
  echo
  echo "Pushed ${IMAGE}:${TAG}. Deploy it with:"
  echo "  $0 --deploy"
  exit 0
fi

echo "==> registering task definition revision"
TMP="$(mktemp)"
jq --arg img "${IMAGE}:${TAG}" '.containerDefinitions[0].image = $img' "$TASKDEF" > "$TMP"
mv "$TMP" "$TASKDEF"
REVISION="$(aws ecs register-task-definition --region "$REGION" \
  --cli-input-json "file://${TASKDEF}" \
  --query 'taskDefinition.revision' --output text)"
echo "    registered ${SERVICE}:${REVISION}"

echo "==> deploying"
aws ecs update-service --region "$REGION" --cluster "$CLUSTER" --service "$SERVICE" \
  --task-definition "${SERVICE}:${REVISION}" --query 'service.taskDefinition' --output text

aws ecs wait services-stable --region "$REGION" --cluster "$CLUSTER" --services "$SERVICE"

# RUNNING only means the port is open; HEALTHY means the container health
# probe confirmed casts can actually reach the commit store.
HEALTH="$(aws ecs list-tasks --region "$REGION" --cluster "$CLUSTER" --service-name "$SERVICE" \
  --query 'taskArns[0]' --output text \
  | xargs -I{} aws ecs describe-tasks --region "$REGION" --cluster "$CLUSTER" --tasks {} \
      --query 'tasks[0].healthStatus' --output text)"
echo
echo "deployed ${SERVICE}:${REVISION}  health=${HEALTH}"
[ "$HEALTH" = HEALTHY ] || { echo "not HEALTHY; check /ecs/casts in CloudWatch" >&2; exit 1; }
