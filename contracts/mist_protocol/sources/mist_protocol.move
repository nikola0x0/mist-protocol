/*
/// Module: mist_protocol
module mist_protocol::mist_protocol;
*/

// For Move coding conventions, see
// https://docs.sui.io/concepts/sui-move-concepts/conventions


module mist_protocol::mist_protocol {
    use sui::coin::{Self, Coin};
    use sui::balance::{Self, Balance};
    use sui::event;
    use sui::sui::SUI;
    use usdc::usdc::USDC;

    // ============ WITNESS (for Nautilus Enclave) ============
    /// One-Time-Witness for creating Enclave capability
    public struct MIST_PROTOCOL has drop {}

    // ============ ERRORS ============
    const E_INSUFFICIENT_BALANCE: u64 = 1;
    const E_NOT_AUTHORIZED: u64 = 2;
    const E_PAUSED: u64 = 3;

    // ============ STRUCTS ============
    public struct LiquidityPool has key {
        id: UID,
        sui_balance: Balance<SUI>,
        usdc_balance: Balance<USDC>,
        tee_authority: address,
        paused: bool,
    }

    public struct EncryptedSUI has key, store {
        id: UID,
        balance_pointer: vector<u8>,
    }

    public struct EncryptedUSDC has key, store {
        id: UID,
        balance_pointer: vector<u8>,
    }

    public struct AdminCap has key {
        id: UID,
    }

    // ============ EVENTS ============
    public struct WrapEvent has copy, drop {
        user: address,
        token_type: vector<u8>,
        amount: u64,
        pointer: vector<u8>,
    }

    public struct UnwrapEvent has copy, drop {
        user: address,
        token_type: vector<u8>,
        amount: u64,
        recipient: address,
    }

    /// Emitted when user requests a swap
    public struct SwapRequestEvent has copy, drop {
        user: address,
        esui_id: ID,
        eusdc_id: ID,
        from_token: vector<u8>,
        to_token: vector<u8>,
        swap_amount_encrypted: vector<u8>,
    }

    /// Emitted when TEE completes a swap on Cetus
    public struct SwapExecutedEvent has copy, drop {
        user: address,
        from_token: vector<u8>,
        to_token: vector<u8>,
        from_amount: u64,
        to_amount: u64,
        timestamp: u64,
    }

    // ============ INIT ============
    fun init(_witness: MIST_PROTOCOL, ctx: &mut TxContext) {
        let pool = LiquidityPool {
            id: object::new(ctx),
            sui_balance: balance::zero(),
            usdc_balance: balance::zero(),
            tee_authority: tx_context::sender(ctx),
            paused: false,
        };
        transfer::share_object(pool);

        let admin_cap = AdminCap {
            id: object::new(ctx),
        };
        transfer::transfer(admin_cap, tx_context::sender(ctx));
    }

    // ============ WRAP FUNCTIONS ============
    /// User deposits SUI and gets eSUI token
    entry fun wrap_sui(
        pool: &mut LiquidityPool,
        payment: Coin<SUI>,
        encrypted_pointer: vector<u8>,
        ctx: &mut TxContext
    ) {
        assert!(!pool.paused, E_PAUSED);
        
        // Get amount
        let amount = coin::value(&payment);
        
        // Lock SUI in pool
        balance::join(&mut pool.sui_balance, coin::into_balance(payment));
        
        // Create eSUI token for user
        let esui = EncryptedSUI {
            id: object::new(ctx),
            balance_pointer: encrypted_pointer,
        };
        
        // Give token to user
        let sender = tx_context::sender(ctx);
        transfer::transfer(esui, sender);
        
        // Emit event
        event::emit(WrapEvent {
            user: sender,
            token_type: b"SUI",
            amount,
            pointer: encrypted_pointer,
        });
    }

    /// User deposits USDC and gets eUSDC token
    entry fun wrap_usdc(
        pool: &mut LiquidityPool,
        payment: Coin<USDC>,
        encrypted_pointer: vector<u8>,
        ctx: &mut TxContext
    ) {
        assert!(!pool.paused, E_PAUSED);
        
        let amount = coin::value(&payment);
        
        // Lock USDC in pool
        balance::join(&mut pool.usdc_balance, coin::into_balance(payment));
        
        // Create eUSDC token
        let eusdc = EncryptedUSDC {
            id: object::new(ctx),
            balance_pointer: encrypted_pointer,
        };
        
        let sender = tx_context::sender(ctx);
        transfer::transfer(eusdc, sender);
        
        event::emit(WrapEvent {
            user: sender,
            token_type: b"USDC",
            amount,
            pointer: encrypted_pointer,
        });
    }

    // ============ MERGE FUNCTIONS ============
    entry fun merge_sui(
        pool: &mut LiquidityPool,
        esui: &mut EncryptedSUI,
        payment: Coin<SUI>,
        new_pointer: vector<u8>,  // Updated encrypted balance
        ctx: &TxContext
    ) {
        assert!(!pool.paused, E_PAUSED);
        
        let amount = coin::value(&payment);
        
        // Add SUI to pool
        balance::join(&mut pool.sui_balance, coin::into_balance(payment));
        
        // Update pointer
        esui.balance_pointer = new_pointer;
        
        event::emit(WrapEvent {
            user: tx_context::sender(ctx),
            token_type: b"SUI",
            amount,
            pointer: new_pointer,
        });
    }

    entry fun merge_usdc(
        pool: &mut LiquidityPool,
        eusdc: &mut EncryptedUSDC,
        payment: Coin<USDC>,
        new_pointer: vector<u8>,
        ctx: &TxContext
    ) {
        assert!(!pool.paused, E_PAUSED);
        
        let amount = coin::value(&payment);
        
        balance::join(&mut pool.usdc_balance, coin::into_balance(payment));
        
        eusdc.balance_pointer = new_pointer;
        
        event::emit(WrapEvent {
            user: tx_context::sender(ctx),
            token_type: b"USDC",
            amount,
            pointer: new_pointer,
        });
    }

    // ============ SWAP FUNCTIONS ============
    /// User requests swap SUI → USDC
    entry fun request_swap_sui_to_usdc(
        esui: &EncryptedSUI,
        eusdc: &EncryptedUSDC,
        swap_amount_encrypted: vector<u8>,
        ctx: &TxContext
    ) {
        event::emit(SwapRequestEvent {
            user: tx_context::sender(ctx),
            esui_id: object::uid_to_inner(&esui.id),
            eusdc_id: object::uid_to_inner(&eusdc.id),
            from_token: b"SUI",
            to_token: b"USDC",
            swap_amount_encrypted,
        });
    }

    /// User requests swap USDC → SUI
    entry fun request_swap_usdc_to_sui(
        eusdc: &EncryptedUSDC,
        esui: &EncryptedSUI,
        swap_amount_encrypted: vector<u8>,
        ctx: &TxContext
    ) {
        event::emit(SwapRequestEvent {
            user: tx_context::sender(ctx),
            esui_id: object::uid_to_inner(&esui.id),
            eusdc_id: object::uid_to_inner(&eusdc.id),
            from_token: b"USDC",
            to_token: b"SUI",
            swap_amount_encrypted,
        });
    }

    /// TEE calls this after executing swap on Cetus: SUI → USDC
    entry fun update_after_swap_sui_to_usdc(
        pool: &LiquidityPool,
        esui: &mut EncryptedSUI,
        eusdc: &mut EncryptedUSDC,
        sui_spent: u64,
        usdc_received: u64,
        new_esui_pointer: vector<u8>,
        new_eusdc_pointer: vector<u8>,
        ctx: &TxContext
    ) {
        // Only TEE can call
        assert!(tx_context::sender(ctx) == pool.tee_authority, E_NOT_AUTHORIZED);
        assert!(!pool.paused, E_PAUSED);
        
        // Update encrypted pointers only (TEE already swapped on Cetus)
        esui.balance_pointer = new_esui_pointer;
        eusdc.balance_pointer = new_eusdc_pointer;
        
        // Emit event for transparency
        event::emit(SwapExecutedEvent {
            user: object::uid_to_address(&esui.id),
            from_token: b"SUI",
            to_token: b"USDC",
            from_amount: sui_spent,
            to_amount: usdc_received,
            timestamp: tx_context::epoch(ctx),
        });
    }

    /// TEE calls this after executing swap on Cetus: USDC → SUI
    entry fun update_after_swap_usdc_to_sui(
        pool: &LiquidityPool,
        eusdc: &mut EncryptedUSDC,
        esui: &mut EncryptedSUI,
        usdc_spent: u64,
        sui_received: u64,
        new_eusdc_pointer: vector<u8>,
        new_esui_pointer: vector<u8>,
        ctx: &TxContext
    ) {
        assert!(tx_context::sender(ctx) == pool.tee_authority, E_NOT_AUTHORIZED);
        assert!(!pool.paused, E_PAUSED);
        
        eusdc.balance_pointer = new_eusdc_pointer;
        esui.balance_pointer = new_esui_pointer;
        
        event::emit(SwapExecutedEvent {
            user: object::uid_to_address(&eusdc.id),
            from_token: b"USDC",
            to_token: b"SUI",
            from_amount: usdc_spent,
            to_amount: sui_received,
            timestamp: tx_context::epoch(ctx),
        });
    }


    // ============ UNWRAP FUNCTIONS ============
    /// User burns eSUI and gets real SUI back
    entry fun unwrap_sui(
        pool: &mut LiquidityPool,
        esui: EncryptedSUI,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        assert!(!pool.paused, E_PAUSED);
        
        // Check pool has enough
        assert!(balance::value(&pool.sui_balance) >= amount, E_INSUFFICIENT_BALANCE);
        
        // Extract SUI from pool
        let sui_to_send = coin::from_balance(
            balance::split(&mut pool.sui_balance, amount),
            ctx
        );
        
        // Send to recipient
        transfer::public_transfer(sui_to_send, recipient);
        
        // Burn the eSUI token
        let EncryptedSUI { id, balance_pointer: _ } = esui;
        object::delete(id);
        
        event::emit(UnwrapEvent {
            user: tx_context::sender(ctx),
            token_type: b"SUI",
            amount,
            recipient,
        });
    }

    /// Partial unwrap - keep some eSUI
    entry fun unwrap_sui_partial(
        pool: &mut LiquidityPool,
        esui: &mut EncryptedSUI,
        amount: u64,
        recipient: address,
        new_pointer: vector<u8>,  // Updated balance after unwrap
        ctx: &mut TxContext
    ) {
        assert!(!pool.paused, E_PAUSED);
        assert!(balance::value(&pool.sui_balance) >= amount, E_INSUFFICIENT_BALANCE);
        
        // Send SUI
        let sui_to_send = coin::from_balance(
            balance::split(&mut pool.sui_balance, amount),
            ctx
        );
        transfer::public_transfer(sui_to_send, recipient);
        
        // Update pointer (don't burn token)
        esui.balance_pointer = new_pointer;
        
        event::emit(UnwrapEvent {
            user: tx_context::sender(ctx),
            token_type: b"SUI",
            amount,
            recipient,
        });
    }

    /// User burns eUSDC and gets real USDC back
    entry fun unwrap_usdc(
        pool: &mut LiquidityPool,
        eusdc: EncryptedUSDC,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        assert!(!pool.paused, E_PAUSED);
        assert!(balance::value(&pool.usdc_balance) >= amount, E_INSUFFICIENT_BALANCE);
        
        let usdc_to_send = coin::from_balance(
            balance::split(&mut pool.usdc_balance, amount),
            ctx
        );
        
        transfer::public_transfer(usdc_to_send, recipient);
        
        let EncryptedUSDC { id, balance_pointer: _ } = eusdc;
        object::delete(id);
        
        event::emit(UnwrapEvent {
            user: tx_context::sender(ctx),
            token_type: b"USDC",
            amount,
            recipient,
        });
    }

    /// Partial unwrap USDC
    entry fun unwrap_usdc_partial(
        pool: &mut LiquidityPool,
        eusdc: &mut EncryptedUSDC,
        amount: u64,
        recipient: address,
        new_pointer: vector<u8>,
        ctx: &mut TxContext
    ) {
        assert!(!pool.paused, E_PAUSED);
        assert!(balance::value(&pool.usdc_balance) >= amount, E_INSUFFICIENT_BALANCE);
        
        let usdc_to_send = coin::from_balance(
            balance::split(&mut pool.usdc_balance, amount),
            ctx
        );
        transfer::public_transfer(usdc_to_send, recipient);
        
        eusdc.balance_pointer = new_pointer;
        
        event::emit(UnwrapEvent {
            user: tx_context::sender(ctx),
            token_type: b"USDC",
            amount,
            recipient,
        });
    }

    // ============ TRANSFER FUNCTIONS ============
    /// Send entire eSUI token to someone
    entry fun transfer_esui(
        esui: EncryptedSUI,
        recipient: address,
    ) {
        transfer::public_transfer(esui, recipient);
    }

    /// Send entire eUSDC token to someone
    entry fun transfer_eusdc(
        eusdc: EncryptedUSDC,
        recipient: address,
    ) {
        transfer::public_transfer(eusdc, recipient);
    }

    /// Split eSUI and send part to someone
    entry fun split_and_send_esui(
        esui: &mut EncryptedSUI,
        sender_new_pointer: vector<u8>,
        recipient_pointer: vector<u8>,
        recipient: address,
        ctx: &mut TxContext
    ) {
        // Update sender's token
        esui.balance_pointer = sender_new_pointer;
        
        // Create new token for recipient
        let recipient_esui = EncryptedSUI {
            id: object::new(ctx),
            balance_pointer: recipient_pointer,
        };
        
        transfer::transfer(recipient_esui, recipient);
    }

    /// Split eUSDC and send part to someone
    entry fun split_and_send_eusdc(
        eusdc: &mut EncryptedUSDC,
        sender_new_pointer: vector<u8>,
        recipient_pointer: vector<u8>,
        recipient: address,
        ctx: &mut TxContext
    ) {
        eusdc.balance_pointer = sender_new_pointer;
        
        let recipient_eusdc = EncryptedUSDC {
            id: object::new(ctx),
            balance_pointer: recipient_pointer,
        };
        
        transfer::transfer(recipient_eusdc, recipient);
    }

    // ============ ADMIN FUNCTIONS ============
    /// Update TEE authority
    entry fun update_tee_authority(
        pool: &mut LiquidityPool,
        _admin_cap: &AdminCap,
        new_authority: address,
    ) {
        pool.tee_authority = new_authority;
    }

    /// Pause/unpause
    entry fun set_pause(
        pool: &mut LiquidityPool,
        _admin_cap: &AdminCap,
        paused: bool,
    ) {
        pool.paused = paused;
    }
}


