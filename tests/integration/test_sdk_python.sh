#!/usr/bin/env bash
# Python SDK smoke tests — verifies the constellation module can be imported
# and that basic types work correctly.
#
# Skips gracefully if the constellation wheel is not installed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== Python SDK Smoke Tests ==="

# Check if Python 3 is available
if ! command -v python3 &>/dev/null; then
    echo "SKIP: python3 not found"
    exit 0
fi

# Check if the constellation module is importable
if ! python3 -c "import constellation" 2>/dev/null; then
    echo "SKIP: constellation module not installed (build the wheel first)"
    exit 0
fi

PASS=0
FAIL=0

run_test() {
    local name="$1"
    local code="$2"
    if python3 -c "$code" 2>&1; then
        echo "  PASS: $name"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $name"
        FAIL=$((FAIL + 1))
    fi
}

# Test 1: Module import
run_test "import constellation" "
import constellation
print('  Module version:', getattr(constellation, '__version__', 'unknown'))
"

# Test 2: Basic type creation
run_test "basic type creation" "
from constellation import AgentConfig
config = AgentConfig(
    homeserver_url='http://localhost:6167',
    username='test-agent',
    password='secret',
)
assert config.homeserver_url == 'http://localhost:6167'
assert config.username == 'test-agent'
"

# Test 3: AgentConfig construction with optional fields
run_test "AgentConfig with optional fields" "
from constellation import AgentConfig
config = AgentConfig(
    homeserver_url='http://localhost:6167',
    username='test-agent',
    password='secret',
    display_name='Test Agent',
    auto_join_rooms=['#room:server'],
)
assert config.display_name == 'Test Agent'
assert len(config.auto_join_rooms) == 1
"

# Test 4: Message creation
run_test "Message creation" "
from constellation import Message
msg = Message.text('Hello, agents!')
assert msg.body == 'Hello, agents!'
assert msg.metadata is None
"

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
