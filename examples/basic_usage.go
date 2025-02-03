package main

import (
	"context"
	"fmt"
	"log"
	"time"

	"github.com/labs-alone/alone-main/internal/core"
	"github.com/labs-alone/alone-main/internal/solana"
	"github.com/labs-alone/alone-main/internal/openai"
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
		utils.WithPrefix("Example"),
	)

	// Initialize core engine
	engine, err := core.NewEngine(config)
	if err != nil {
		logger.Fatal("Failed to initialize engine", map[string]interface{}{
			"error": err.Error(),
		})
	}
	defer engine.Shutdown(context.Background())

	// Initialize Solana client
	solanaClient, err := solana.NewClient(config.Solana)
	if err != nil {
		logger.Fatal("Failed to initialize Solana client", map[string]interface{}{
			"error": err.Error(),
		})
	}

	// Initialize OpenAI client
	openaiClient, err := openai.NewClient(&openai.ClientConfig{
		APIKey:  config.OpenAI.APIKey,
		Timeout: 30 * time.Second,
	})
	if err != nil {
		logger.Fatal("Failed to initialize OpenAI client", map[string]interface{}{
			"error": err.Error(),
		})
	}

	// Example 1: Check Solana balance
	fmt.Println("\n=== Example 1: Check Solana Balance ===")
	balance, err := solanaClient.GetBalance(context.Background(), "your_address_here")
	if err != nil {
		logger.Error("Failed to get balance", map[string]interface{}{
			"error": err.Error(),
		})
	} else {
		fmt.Printf("Balance: %d lamports\n", balance)
	}

	// Example 2: Send AI completion request
	fmt.Println("\n=== Example 2: AI Completion ===")
	completion, err := openaiClient.CreateChatCompletion(
		context.Background(),
		&openai.ChatCompletionRequest{
			Messages: []openai.ChatMessage{
				{
					Role:    "user",
					Content: "What is the Solana blockchain?",
				},
			},
			MaxTokens:   100,
			Temperature: 0.7,
		},
	)
	if err != nil {
		logger.Error("Failed to get completion", map[string]interface{}{
			"error": err.Error(),
		})
	} else {
		fmt.Printf("AI Response: %s\n", completion.Choices[0].Message.Content)
	}

	// Example 3: Process custom request
	fmt.Println("\n=== Example 3: Custom Request ===")
	request := &core.Request{
		ID:   "example-request",
		Type: "custom",
		Payload: map[string]interface{}{
			"action": "process",
			"data":   "example data",
		},
	}

	result, err := engine.ProcessRequest(request)
	if err != nil {
		logger.Error("Failed to process request", map[string]interface{}{
			"error": err.Error(),
		})
	} else {
		fmt.Printf("Request processed: %+v\n", result)
	}

	// Example 4: State management
	fmt.Println("\n=== Example 4: State Management ===")
	err = engine.UpdateState("active", map[string]interface{}{
		"last_update": time.Now(),
		"status":      "running",
	})
	if err != nil {
		logger.Error("Failed to update state", map[string]interface{}{
			"error": err.Error(),
		})
	}

	state := engine.GetState()
	fmt.Printf("Current state: %+v\n", state)

	// Example 5: Metrics
	fmt.Println("\n=== Example 5: Metrics ===")
	engineMetrics := engine.GetMetrics()
	solanaMetrics := solanaClient.GetMetrics()
	openaiMetrics := openaiClient.GetMetrics()

	fmt.Printf("Engine Metrics: %+v\n", engineMetrics)
	fmt.Printf("Solana Metrics: %+v\n", solanaMetrics)
	fmt.Printf("OpenAI Metrics: %+v\n", openaiMetrics)

	// Example 6: Error handling
	fmt.Println("\n=== Example 6: Error Handling ===")
	_, err = solanaClient.GetBalance(context.Background(), "invalid_address")
	if err != nil {
		fmt.Printf("Expected error handled: %v\n", err)
	}

	// Example 7: Concurrent requests
	fmt.Println("\n=== Example 7: Concurrent Requests ===")
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	results := make(chan error, 3)
	for i := 0; i < 3; i++ {
		go func(id int) {
			_, err := solanaClient.GetBalance(ctx, "address")
			results <- err
		}(i)
	}

	for i := 0; i < 3; i++ {
		if err := <-results; err != nil {
			fmt.Printf("Concurrent request %d failed: %v\n", i, err)
		} else {
			fmt.Printf("Concurrent request %d succeeded\n", i)
		}
	}

	fmt.Println("\nExamples completed successfully!")
}