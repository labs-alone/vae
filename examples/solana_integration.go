package main

import (
	"context"
	"fmt"
	"log"
	"time"

	"github.com/labs-alone/alone-main/internal/solana"
	"github.com/labs-alone/alone-main/internal/utils"
)

func main() {
	// Load configuration
	config, err := utils.LoadConfig("../config/config.yaml")
	if err != nil {
		log.Fatalf("Failed to load config: %v", err)
	}

	// Initialize logger
	logger := utils.NewLogger(
		utils.WithLevel(utils.INFO),
		utils.WithPrefix("SolanaExample"),
	)

	// Initialize Solana client
	client, err := solana.NewClient(config.Solana)
	if err != nil {
		logger.Fatal("Failed to initialize Solana client", map[string]interface{}{
			"error": err.Error(),
		})
	}

	// Example 1: Account Management
	fmt.Println("\n=== Example 1: Account Management ===")
	demonstrateAccountManagement(client, logger)

	// Example 2: Transaction Handling
	fmt.Println("\n=== Example 2: Transaction Handling ===")
	demonstrateTransactionHandling(client, logger)

	// Example 3: Program Interaction
	fmt.Println("\n=== Example 3: Program Interaction ===")
	demonstrateProgramInteraction(client, logger)

	// Example 4: Token Operations
	fmt.Println("\n=== Example 4: Token Operations ===")
	demonstrateTokenOperations(client, logger)

	// Example 5: WebSocket Subscriptions
	fmt.Println("\n=== Example 5: WebSocket Subscriptions ===")
	demonstrateWebSocketSubscriptions(client, logger)

	fmt.Println("\nSolana integration examples completed!")
}

func demonstrateAccountManagement(client *solana.Client, logger *utils.Logger) {
	ctx := context.Background()

	// Create new wallet
	wallet, err := client.CreateWallet()
	if err != nil {
		logger.Error("Failed to create wallet", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Created new wallet: %s\n", wallet.PublicKey())

	// Check balance
	balance, err := client.GetBalance(ctx, wallet.PublicKey())
	if err != nil {
		logger.Error("Failed to get balance", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Wallet balance: %d lamports\n", balance)

	// Get account info
	info, err := client.GetAccountInfo(ctx, wallet.PublicKey())
	if err != nil {
		logger.Error("Failed to get account info", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Account info: %+v\n", info)
}

func demonstrateTransactionHandling(client *solana.Client, logger *utils.Logger) {
	ctx := context.Background()

	// Create test wallets
	sender, _ := client.CreateWallet()
	recipient, _ := client.CreateWallet()

	// Request airdrop for testing
	signature, err := client.RequestAirdrop(ctx, sender.PublicKey(), 1000000000) // 1 SOL
	if err != nil {
		logger.Error("Failed to request airdrop", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Airdrop requested: %s\n", signature)

	// Wait for confirmation
	err = client.ConfirmTransaction(ctx, signature, "confirmed")
	if err != nil {
		logger.Error("Failed to confirm airdrop", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}

	// Send transaction
	tx, err := client.SendTransaction(
		ctx,
		sender.PublicKey(),
		recipient.PublicKey(),
		500000000, // 0.5 SOL
	)
	if err != nil {
		logger.Error("Failed to send transaction", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Transaction sent: %s\n", tx)

	// Get transaction status
	status, err := client.GetTransactionStatus(ctx, tx)
	if err != nil {
		logger.Error("Failed to get transaction status", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Transaction status: %s\n", status)
}

func demonstrateProgramInteraction(client *solana.Client, logger *utils.Logger) {
	ctx := context.Background()

	// Example program ID
	programID := "Your_Program_ID"

	// Create program account
	account, err := client.CreateProgramAccount(ctx, programID)
	if err != nil {
		logger.Error("Failed to create program account", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Program account created: %s\n", account)

	// Interact with program
	instruction := []byte("example_instruction")
	signature, err := client.SendProgramInstruction(ctx, programID, instruction)
	if err != nil {
		logger.Error("Failed to send program instruction", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Program instruction sent: %s\n", signature)
}

func demonstrateTokenOperations(client *solana.Client, logger *utils.Logger) {
	ctx := context.Background()

	// Create token mint
	mint, err := client.CreateTokenMint(ctx)
	if err != nil {
		logger.Error("Failed to create token mint", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Token mint created: %s\n", mint)

	// Create token account
	account, err := client.CreateTokenAccount(ctx, mint)
	if err != nil {
		logger.Error("Failed to create token account", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Token account created: %s\n", account)

	// Mint tokens
	signature, err := client.MintTokens(ctx, mint, account, 1000)
	if err != nil {
		logger.Error("Failed to mint tokens", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Tokens minted: %s\n", signature)
}

func demonstrateWebSocketSubscriptions(client *solana.Client, logger *utils.Logger) {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// Subscribe to account changes
	accountSub, err := client.SubscribeToAccountChanges(ctx, "your_account_address")
	if err != nil {
		logger.Error("Failed to subscribe to account", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Subscribed to account changes: %s\n", accountSub)

	// Subscribe to program
	programSub, err := client.SubscribeToProgram(ctx, "your_program_id")
	if err != nil {
		logger.Error("Failed to subscribe to program", map[string]interface{}{
			"error": err.Error(),
		})
		return
	}
	fmt.Printf("Subscribed to program: %s\n", programSub)

	// Wait for some notifications
	time.Sleep(10 * time.Second)

	// Unsubscribe
	if err := client.Unsubscribe(ctx, accountSub); err != nil {
		logger.Error("Failed to unsubscribe from account", map[string]interface{}{
			"error": err.Error(),
		})
	}

	if err := client.Unsubscribe(ctx, programSub); err != nil {
		logger.Error("Failed to unsubscribe from program", map[string]interface{}{
			"error": err.Error(),
		})
	}
}