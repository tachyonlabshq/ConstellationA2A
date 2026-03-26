#!/usr/bin/env bash
# Docker build validation tests.
#
# Tests that Docker Compose configs validate and Dockerfiles build correctly.
# Set SKIP_DOCKER_BUILD=1 to skip image builds (which can be slow).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== Docker Build Validation Tests ==="

PASS=0
FAIL=0
SKIP=0

# Check if Docker is available
if ! command -v docker &>/dev/null; then
    echo "SKIP: docker not found"
    exit 0
fi

run_test() {
    local name="$1"
    shift
    if "$@" &>/dev/null; then
        echo "  PASS: $name"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $name"
        FAIL=$((FAIL + 1))
    fi
}

skip_test() {
    local name="$1"
    echo "  SKIP: $name"
    SKIP=$((SKIP + 1))
}

# Test 1: docker-compose.yml validates
run_test "docker-compose.yml validates" \
    docker compose -f "$PROJECT_ROOT/docker-compose.yml" config --quiet

# Test 2: docker-compose.prod.yml overlay validates
if [ -f "$PROJECT_ROOT/docker-compose.prod.yml" ]; then
    run_test "docker-compose.prod.yml overlay validates" \
        docker compose \
            -f "$PROJECT_ROOT/docker-compose.yml" \
            -f "$PROJECT_ROOT/docker-compose.prod.yml" \
            config --quiet
else
    skip_test "docker-compose.prod.yml overlay validates (file not found)"
fi

# Test 3: Conduit Dockerfile builds
if [ "${SKIP_DOCKER_BUILD:-0}" = "1" ]; then
    skip_test "Conduit Dockerfile builds (SKIP_DOCKER_BUILD=1)"
else
    if [ -f "$PROJECT_ROOT/conduit/Dockerfile" ]; then
        run_test "Conduit Dockerfile builds" \
            docker build -t constellation-conduit-test "$PROJECT_ROOT/conduit"
    else
        skip_test "Conduit Dockerfile builds (Dockerfile not found)"
    fi
fi

# Test 4: Base agent Dockerfile builds (skippable — can be slow)
if [ "${SKIP_DOCKER_BUILD:-0}" = "1" ]; then
    skip_test "Base agent Dockerfile builds (SKIP_DOCKER_BUILD=1)"
else
    if [ -f "$PROJECT_ROOT/agents/Dockerfile" ]; then
        run_test "Base agent Dockerfile builds" \
            docker build -t constellation-agent-test "$PROJECT_ROOT/agents"
    else
        skip_test "Base agent Dockerfile builds (Dockerfile not found)"
    fi
fi

echo ""
echo "=== Results: $PASS passed, $FAIL failed, $SKIP skipped ==="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
