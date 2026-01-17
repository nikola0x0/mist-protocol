#!/bin/bash

# TwitterAPI.io Interactive CLI Management Tool

set -e

# Load API key from .env file or environment
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="$SCRIPT_DIR/../.env"

if [ -f "$ENV_FILE" ]; then
    export $(grep -E '^TWITTERAPI_IO_KEY=' "$ENV_FILE" | xargs)
fi

API_KEY="${TWITTERAPI_IO_KEY:-}"
BASE_URL="https://api.twitterapi.io/oapi"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

# Check if API key is set
check_api_key() {
    if [ -z "$API_KEY" ]; then
        echo -e "${RED}Error: TWITTERAPI_IO_KEY not set${NC}"
        echo "Set it in .env file or export TWITTERAPI_IO_KEY=your_key"
        exit 1
    fi
}

# Clear screen and show header
show_header() {
    clear
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}       ${BOLD}TwitterAPI.io Filter Rules Manager${NC}                    ${CYAN}║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

# Make API request
api_request() {
    local method="$1"
    local endpoint="$2"
    local data="$3"

    if [ "$method" = "GET" ]; then
        curl -s "${BASE_URL}${endpoint}" -H "x-api-key: $API_KEY"
    else
        curl -s -X POST "${BASE_URL}${endpoint}" \
            -H "x-api-key: $API_KEY" \
            -H "Content-Type: application/json" \
            -d "$data"
    fi
}

# Show account info
show_info() {
    echo -e "${BLUE}Fetching account info...${NC}"
    local response=$(api_request GET "/my/info")

    local recharge=$(echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('recharge_credits', 0))" 2>/dev/null)
    local bonus=$(echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('total_bonus_credits', 0))" 2>/dev/null)
    local total=$((recharge + bonus))

    echo ""
    echo -e "  ${BOLD}Account Credits${NC}"
    echo -e "  ─────────────────────────"
    echo -e "  Recharge:  ${GREEN}$(printf "%'d" $recharge)${NC}"
    echo -e "  Bonus:     ${GREEN}$(printf "%'d" $bonus)${NC}"
    echo -e "  ─────────────────────────"
    echo -e "  Total:     ${BOLD}${GREEN}$(printf "%'d" $total)${NC}"
    echo ""
}

# Fetch and cache rules
RULES_CACHE=""
fetch_rules() {
    RULES_CACHE=$(api_request GET "/tweet_filter/get_rules")
}

# Get rule count
get_rule_count() {
    echo "$RULES_CACHE" | python3 -c "import sys,json; print(len(json.load(sys.stdin).get('rules', [])))" 2>/dev/null
}

# Toggle rule (activate/deactivate)
toggle_rule() {
    local idx="$1"
    local new_effect="$2"  # 1 = activate, 0 = deactivate

    local rule_data=$(echo "$RULES_CACHE" | python3 -c "
import sys, json
idx = int('$idx') - 1
data = json.load(sys.stdin)
rules = data.get('rules', [])
if 0 <= idx < len(rules):
    r = rules[idx]
    print(f\"{r['rule_id']}|{r['tag']}|{r['value']}|{int(r['interval_seconds'])}\")
")

    if [ -n "$rule_data" ]; then
        IFS='|' read -r rule_id tag value interval <<< "$rule_data"
        local action=$( [ "$new_effect" = "1" ] && echo "Activating" || echo "Deactivating" )
        echo -n "  $action $tag... "
        local result=$(api_request POST "/tweet_filter/update_rule" "{\"rule_id\":\"$rule_id\",\"tag\":\"$tag\",\"value\":\"$value\",\"interval_seconds\":$interval,\"is_effect\":$new_effect}")
        if echo "$result" | grep -q '"success"'; then
            echo -e "${GREEN}✓${NC}"
            return 0
        else
            echo -e "${RED}✗${NC}"
            return 1
        fi
    fi
    return 1
}

# Toggle all rules
toggle_all_rules() {
    local new_effect="$1"
    local action=$( [ "$new_effect" = "1" ] && echo "Activating" || echo "Deactivating" )

    echo -e "  ${YELLOW}$action all rules...${NC}"
    echo ""

    # Use python to handle JSON properly (avoids quote escaping issues)
    echo "$RULES_CACHE" | python3 -c "
import sys, json, subprocess

new_effect = $new_effect
action = '$action'
data = json.load(sys.stdin)
rules = data.get('rules', [])

for r in rules:
    print(f\"  {action} {r['tag']}... \", end='', flush=True)
    payload = json.dumps({
        'rule_id': r['rule_id'],
        'tag': r['tag'],
        'value': r['value'],
        'interval_seconds': int(r['interval_seconds']),
        'is_effect': new_effect
    })
    result = subprocess.run([
        'curl', '-s', '-X', 'POST',
        'https://api.twitterapi.io/oapi/tweet_filter/update_rule',
        '-H', 'x-api-key: $API_KEY',
        '-H', 'Content-Type: application/json',
        '-d', payload
    ], capture_output=True, text=True)
    if 'success' in result.stdout:
        print('\033[92m✓\033[0m')
    else:
        print('\033[91m✗\033[0m')
"
}

# List rules with inline actions
list_rules_interactive() {
    while true; do
        show_header
        echo -e "  ${BOLD}Manage Rules${NC}"
        echo ""

        echo -e "  ${DIM}Fetching...${NC}"
        fetch_rules

        # Move cursor up to overwrite "Fetching..."
        echo -e "\033[1A\033[2K"

        local count=$(get_rule_count)

        if [ "$count" -eq 0 ]; then
            echo -e "  ${DIM}No rules found${NC}"
            echo ""
            echo -e "  ${DIM}Press Enter to go back${NC}"
            read
            return
        fi

        echo "$RULES_CACHE" | python3 -c "
import sys, json
data = json.load(sys.stdin)
rules = data.get('rules', [])
print(f'  Found {len(rules)} rule(s):\n')
for i, r in enumerate(rules, 1):
    if r['is_effect'] == 1:
        status = '\033[92m● ON \033[0m'
    else:
        status = '\033[91m○ OFF\033[0m'
    print(f\"  {i}. {status}  \033[1m{r['tag']}\033[0m\")
    print(f\"            Interval: {int(r['interval_seconds'])}s | Query: \033[2m{r['value']}\033[0m\")
    print()
"
        echo -e "  ─────────────────────────────────────────────────────────"
        echo -e "  ${BOLD}Actions:${NC}"
        echo -e "    ${CYAN}<number>${NC}      Toggle rule on/off"
        echo -e "    ${CYAN}on <n>${NC}       Activate rule #n      ${CYAN}on all${NC}   Activate all"
        echo -e "    ${CYAN}off <n>${NC}      Deactivate rule #n    ${CYAN}off all${NC}  Deactivate all"
        echo -e "    ${CYAN}e <n>${NC}        Edit rule #n"
        echo -e "    ${CYAN}r${NC}            Refresh list"
        echo -e "    ${CYAN}b${NC}            Back to main menu"
        echo ""
        read -p "  > " cmd arg

        case "$cmd" in
            [1-9]|[1-9][0-9])
                # Toggle single rule
                if [ "$cmd" -le "$count" ]; then
                    local current_effect=$(echo "$RULES_CACHE" | python3 -c "
import sys, json
idx = int('$cmd') - 1
data = json.load(sys.stdin)
rules = data.get('rules', [])
if 0 <= idx < len(rules):
    print(rules[idx]['is_effect'])
")
                    local new_effect=$( [ "$current_effect" = "1" ] && echo "0" || echo "1" )
                    toggle_rule "$cmd" "$new_effect"
                    sleep 0.5
                fi
                ;;
            on)
                if [ "$arg" = "all" ]; then
                    toggle_all_rules 1
                    echo ""
                    read -p "  Press Enter to continue..."
                elif [ -n "$arg" ] && [ "$arg" -le "$count" ] 2>/dev/null; then
                    toggle_rule "$arg" 1
                    sleep 0.5
                fi
                ;;
            off)
                if [ "$arg" = "all" ]; then
                    toggle_all_rules 0
                    echo ""
                    read -p "  Press Enter to continue..."
                elif [ -n "$arg" ] && [ "$arg" -le "$count" ] 2>/dev/null; then
                    toggle_rule "$arg" 0
                    sleep 0.5
                fi
                ;;
            e|edit)
                if [ -n "$arg" ] && [ "$arg" -le "$count" ] 2>/dev/null; then
                    edit_rule_inline "$arg"
                fi
                ;;
            r|refresh)
                # Just loop again to refresh
                ;;
            b|back|q)
                return
                ;;
        esac
    done
}

# Edit rule inline
edit_rule_inline() {
    local idx="$1"

    local rule_data=$(echo "$RULES_CACHE" | python3 -c "
import sys, json
idx = int('$idx') - 1
data = json.load(sys.stdin)
rules = data.get('rules', [])
if 0 <= idx < len(rules):
    r = rules[idx]
    print(f\"{r['rule_id']}|{r['tag']}|{r['value']}|{int(r['interval_seconds'])}|{r['is_effect']}\")
")

    if [ -z "$rule_data" ]; then
        return
    fi

    IFS='|' read -r rule_id tag value interval is_effect <<< "$rule_data"

    show_header
    echo -e "  ${BOLD}Edit Rule: $tag${NC}"
    echo -e "  ─────────────────────────"
    echo ""
    echo -e "  Current settings:"
    echo -e "    Interval: ${CYAN}${interval}s${NC}"
    echo -e "    Query:    ${DIM}$value${NC}"
    echo ""
    echo -e "  ${DIM}Leave blank to keep current value${NC}"
    echo ""

    read -p "  New interval (seconds): " new_interval
    read -p "  New query: " new_query

    [ -z "$new_interval" ] && new_interval="$interval"
    [ -z "$new_query" ] && new_query="$value"

    echo ""
    echo -n "  Updating $tag... "
    local result=$(api_request POST "/tweet_filter/update_rule" "{\"rule_id\":\"$rule_id\",\"tag\":\"$tag\",\"value\":\"$new_query\",\"interval_seconds\":$new_interval,\"is_effect\":$is_effect}")
    if echo "$result" | grep -q '"success"'; then
        echo -e "${GREEN}✓ Done${NC}"
    else
        echo -e "${RED}✗ Failed${NC}"
    fi

    echo ""
    read -p "  Press Enter to continue..."
}

# Add new rule
add_rule_menu() {
    show_header
    echo -e "  ${BOLD}Add New Rule${NC}"
    echo -e "  ─────────────────────────"
    echo ""

    read -p "  Tag (name): " tag
    if [ -z "$tag" ]; then
        echo -e "  ${RED}Tag is required${NC}"
        read -p "  Press Enter to continue..."
        return
    fi

    read -p "  Query: " query
    if [ -z "$query" ]; then
        echo -e "  ${RED}Query is required${NC}"
        read -p "  Press Enter to continue..."
        return
    fi

    read -p "  Interval (seconds) [60]: " interval
    [ -z "$interval" ] && interval=60

    echo ""
    echo -n "  Adding rule '$tag'... "
    local result=$(api_request POST "/tweet_filter/add_rule" "{\"tag\":\"$tag\",\"value\":\"$query\",\"interval_seconds\":$interval}")
    if echo "$result" | grep -q '"success"'; then
        echo -e "${GREEN}✓ Done${NC}"
        echo -e "  ${DIM}Note: New rules are inactive by default.${NC}"
    else
        echo -e "${RED}✗ Failed${NC}"
        echo "  $result"
    fi

    echo ""
    read -p "  Press Enter to continue..."
}

# Delete a rule
delete_rule_menu() {
    show_header
    echo -e "  ${BOLD}Delete Rule${NC}"
    echo -e "  ─────────────────────────"

    fetch_rules
    local count=$(get_rule_count)

    if [ "$count" -eq 0 ]; then
        echo -e "  ${DIM}No rules found${NC}"
        read -p "  Press Enter to continue..."
        return
    fi

    echo ""
    echo "$RULES_CACHE" | python3 -c "
import sys, json
data = json.load(sys.stdin)
rules = data.get('rules', [])
for i, r in enumerate(rules, 1):
    if r['is_effect'] == 1:
        status = '\033[92m●\033[0m'
    else:
        status = '\033[91m○\033[0m'
    print(f\"  {i}. {status} {r['tag']}\")
"
    echo ""
    read -p "  Select rule to delete (or 'b' to go back): " choice

    if [ "$choice" = "b" ] || [ "$choice" = "B" ]; then
        return
    fi

    if ! [ "$choice" -le "$count" ] 2>/dev/null; then
        return
    fi

    local rule_data=$(echo "$RULES_CACHE" | python3 -c "
import sys, json
idx = int('$choice') - 1
data = json.load(sys.stdin)
rules = data.get('rules', [])
if 0 <= idx < len(rules):
    r = rules[idx]
    print(f\"{r['rule_id']}|{r['tag']}\")
")

    if [ -z "$rule_data" ]; then
        return
    fi

    IFS='|' read -r rule_id tag <<< "$rule_data"

    echo ""
    echo -e "  ${YELLOW}Are you sure you want to delete '$tag'?${NC}"
    read -p "  Type 'yes' to confirm: " confirm

    if [ "$confirm" = "yes" ]; then
        echo ""
        echo -n "  Deleting $tag... "
        local result=$(api_request POST "/tweet_filter/delete_rule" "{\"rule_id\":\"$rule_id\"}")
        if echo "$result" | grep -q '"success"'; then
            echo -e "${GREEN}✓ Done${NC}"
        else
            echo -e "${RED}✗ Failed${NC}"
        fi
    else
        echo -e "  ${DIM}Cancelled${NC}"
    fi

    echo ""
    read -p "  Press Enter to continue..."
}

# Main menu
main_menu() {
    while true; do
        show_header

        echo -e "  ${BOLD}Main Menu${NC}"
        echo ""
        echo -e "  ${CYAN}1${NC}  Check Credits"
        echo -e "  ${CYAN}2${NC}  Manage Rules ${DIM}(list, activate, deactivate, edit)${NC}"
        echo -e "  ${CYAN}3${NC}  Add Rule"
        echo -e "  ${CYAN}4${NC}  Delete Rule"
        echo ""
        echo -e "  ${CYAN}q${NC}  Quit"
        echo ""
        read -p "  Select option: " option

        case $option in
            1)
                show_header
                show_info
                read -p "  Press Enter to continue..."
                ;;
            2)
                list_rules_interactive
                ;;
            3)
                add_rule_menu
                ;;
            4)
                delete_rule_menu
                ;;
            q|Q)
                clear
                echo -e "${GREEN}Goodbye!${NC}"
                exit 0
                ;;
            *)
                ;;
        esac
    done
}

# Entry point
check_api_key
main_menu
