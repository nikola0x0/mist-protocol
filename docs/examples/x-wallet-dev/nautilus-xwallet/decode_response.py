#!/usr/bin/env python3
"""
Decode XWallet Enclave Response
Usage: python3 decode_response.py <response.json>
"""

import json
import sys

def decode_bytes(byte_array):
    """Convert byte array to string"""
    return bytes(byte_array).decode('utf-8')

def decode_response(response_json):
    """Decode enclave response"""
    data = response_json['response']['data']

    from_xid = decode_bytes(data['from_xid'])
    to_xid = decode_bytes(data['to_xid'])
    amount_mist = data['amount']
    amount_sui = amount_mist / 1_000_000_000
    coin_type = decode_bytes(data['coin_type'])

    timestamp_ms = response_json['response']['timestamp_ms']
    intent = response_json['response']['intent']
    signature = response_json['signature']

    print("=" * 60)
    print("XWallet Transfer Payload (Decoded)")
    print("=" * 60)
    print(f"From XID:       {from_xid}")
    print(f"To XID:         {to_xid}")
    print(f"Amount (MIST):  {amount_mist:,}")
    print(f"Amount (SUI):   {amount_sui}")
    print(f"Coin Type:      {coin_type}")
    print("-" * 60)
    print(f"Intent:         {intent}")
    print(f"Timestamp (ms): {timestamp_ms}")
    print(f"Signature:      {signature[:32]}...{signature[-32:]}")
    print("=" * 60)
    print("\nReady to submit to Sui blockchain!\n")

if __name__ == "__main__":
    if len(sys.argv) > 1:
        # Read from file
        with open(sys.argv[1], 'r') as f:
            response = json.load(f)
    else:
        # Read from stdin
        print("Paste JSON response (Ctrl+D when done):")
        response = json.load(sys.stdin)

    decode_response(response)
