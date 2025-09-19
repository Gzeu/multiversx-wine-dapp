#![no_std]

use multiversx_sc::derive_imports::*;
use multiversx_sc::imports::*;

#[derive(TopEncode, TopDecode, TypeAbi, PartialEq, Debug, Clone)]
pub struct Listing<M: ManagedTypeApi> {
    pub wine_nft_id: u32,
    pub nft_token_id: TokenIdentifier<M>,
    pub nft_nonce: u64,
    pub seller: ManagedAddress<M>,
    pub price: BigUint<M>,
    pub payment_token: EgldOrEsdtTokenIdentifier<M>,
    pub deadline: u64,
    pub active: bool,
    pub created_timestamp: u64,
}

#[derive(TopEncode, TopDecode, TypeAbi, PartialEq, Debug)]
pub struct Auction<M: ManagedTypeApi> {
    pub wine_nft_id: u32,
    pub nft_token_id: TokenIdentifier<M>,
    pub nft_nonce: u64,
    pub seller: ManagedAddress<M>,
    pub starting_price: BigUint<M>,
    pub current_bid: BigUint<M>,
    pub highest_bidder: ManagedAddress<M>,
    pub payment_token: EgldOrEsdtTokenIdentifier<M>,
    pub end_timestamp: u64,
    pub active: bool,
    pub min_bid_increment: BigUint<M>,
    pub bid_count: u32,
}

#[derive(TopEncode, TopDecode, TypeAbi, PartialEq, Debug)]
pub struct MarketplaceStats<M: ManagedTypeApi> {
    pub total_listings: u32,
    pub total_sales: u32,
    pub total_volume: BigUint<M>,
    pub total_fees_collected: BigUint<M>,
    pub active_listings: u32,
    pub active_auctions: u32,
}

#[multiversx_sc::contract]
pub trait WineMarketplace {
    #[init]
    fn init(
        &self,
        marketplace_fee_percent: u64, // basis points (250 = 2.5%)
        wine_registry_address: ManagedAddress,
    ) {
        require!(marketplace_fee_percent <= 1000, "Fee cannot exceed 10%"); // Max 10%
        self.marketplace_fee_percent().set(marketplace_fee_percent);
        self.wine_registry_address().set(&wine_registry_address);
        self.listing_counter().set(1u32);
        self.auction_counter().set(1u32);
        
        // Initialize supported payment tokens
        let egld_token = EgldOrEsdtTokenIdentifier::egld();
        self.supported_payment_tokens(&egld_token).set(true);
        
        // Initialize marketplace stats
        let stats = MarketplaceStats {
            total_listings: 0u32,
            total_sales: 0u32,
            total_volume: BigUint::zero(),
            total_fees_collected: BigUint::zero(),
            active_listings: 0u32,
            active_auctions: 0u32,
        };
        self.marketplace_stats().set(&stats);
    }

    // Add supported payment token
    #[only_owner]
    #[endpoint(addSupportedToken)]
    fn add_supported_token(&self, token_id: EgldOrEsdtTokenIdentifier) {
        self.supported_payment_tokens(&token_id).set(true);
        self.token_added_event(&token_id);
    }

    // Create NFT listing with escrow
    #[payable("*")]
    #[endpoint(createListing)]
    fn create_listing(
        &self,
        wine_nft_id: u32,
        price: BigUint,
        payment_token: EgldOrEsdtTokenIdentifier,
        duration_seconds: u64,
    ) -> u32 {
        // Validate payment token
        require!(
            self.supported_payment_tokens(&payment_token).get(),
            "Payment token not supported"
        );
        require!(!price.is_zero(), "Price must be greater than zero");
        require!(duration_seconds >= 3600, "Minimum listing duration is 1 hour"); // 1 hour
        require!(duration_seconds <= 2592000, "Maximum listing duration is 30 days"); // 30 days

        let caller = self.blockchain().get_caller();
        let current_timestamp = self.blockchain().get_block_timestamp();
        let deadline = current_timestamp + duration_seconds;

        // Receive NFT in escrow
        let payment = self.call_value().single_esdt();
        let nft_token_id = payment.token_identifier.clone();
        let nft_nonce = payment.token_nonce;
        
        require!(payment.amount == BigUint::from(1u32), "Must send exactly 1 NFT");
        
        // Verify wine ownership through registry (cross-contract call would go here)
        // For now, we trust the NFT transfer as proof of ownership

        let listing_id = self.listing_counter().get();
        let listing = Listing {
            wine_nft_id,
            nft_token_id: nft_token_id.clone(),
            nft_nonce,
            seller: caller.clone(),
            price,
            payment_token: payment_token.clone(),
            deadline,
            active: true,
            created_timestamp: current_timestamp,
        };

        self.listings(listing_id).set(&listing);
        self.listing_counter().set(listing_id + 1);
        self.seller_listings(&caller).push(&listing_id);
        
        // Update stats
        self.marketplace_stats().update(|stats| {
            stats.total_listings += 1;
            stats.active_listings += 1;
        });

        self.wine_listed_event(
            listing_id,
            wine_nft_id,
            &caller,
            &listing.price,
            &payment_token
        );

        listing_id
    }

    // Buy wine from listing
    #[payable("*")]
    #[endpoint(buyWine)]
    fn buy_wine(&self, listing_id: u32) {
        let mut listing = self.listings(listing_id).get();
        require!(listing.active, "Listing is not active");
        require!(
            self.blockchain().get_block_timestamp() <= listing.deadline,
            "Listing has expired"
        );

        let payment = self.call_value().egld_or_single_esdt();
        require!(
            payment.token_identifier == listing.payment_token,
            "Invalid payment token"
        );
        require!(payment.amount >= listing.price, "Insufficient payment");

        let buyer = self.blockchain().get_caller();
        require!(buyer != listing.seller, "Cannot buy your own listing");

        // Calculate fees
        let marketplace_fee = &listing.price * self.marketplace_fee_percent().get() / 10000u64;
        let seller_amount = &listing.price - &marketplace_fee;

        // Transfer NFT to buyer
        self.send().direct_esdt(
            &buyer,
            &listing.nft_token_id,
            listing.nft_nonce,
            &BigUint::from(1u32),
        );

        // Transfer payment to seller
        if listing.payment_token.is_egld() {
            self.send().direct_egld(&listing.seller, &seller_amount);
        } else {
            let token_id = listing.payment_token.unwrap_esdt();
            self.send().direct_esdt(&listing.seller, &token_id, 0, &seller_amount);
        }

        // Collect marketplace fee
        if !marketplace_fee.is_zero() {
            let owner = self.blockchain().get_owner_address();
            if listing.payment_token.is_egld() {
                self.send().direct_egld(&owner, &marketplace_fee);
            } else {
                let token_id = listing.payment_token.unwrap_esdt();
                self.send().direct_esdt(&owner, &token_id, 0, &marketplace_fee);
            }
        }

        // Return surplus if any
        let surplus = &payment.amount - &listing.price;
        if surplus > 0 {
            if payment.token_identifier.is_egld() {
                self.send().direct_egld(&buyer, &surplus);
            } else {
                let token_id = payment.token_identifier.unwrap_esdt();
                self.send().direct_esdt(&buyer, &token_id, 0, &surplus);
            }
        }

        // Deactivate listing
        listing.active = false;
        self.listings(listing_id).set(&listing);
        
        // Update stats
        self.marketplace_stats().update(|stats| {
            stats.total_sales += 1;
            stats.total_volume += &listing.price;
            stats.total_fees_collected += &marketplace_fee;
            stats.active_listings -= 1;
        });

        self.wine_sold_event(
            listing_id,
            listing.wine_nft_id,
            &listing.seller,
            &buyer,
            &listing.price
        );
    }

    // Create auction
    #[payable("*")]
    #[endpoint(createAuction)]
    fn create_auction(
        &self,
        wine_nft_id: u32,
        starting_price: BigUint,
        payment_token: EgldOrEsdtTokenIdentifier,
        duration_seconds: u64,
        min_bid_increment: BigUint,
    ) -> u32 {
        require!(
            self.supported_payment_tokens(&payment_token).get(),
            "Payment token not supported"
        );
        require!(!starting_price.is_zero(), "Starting price must be greater than zero");
        require!(duration_seconds >= 3600, "Minimum auction duration is 1 hour");
        require!(duration_seconds <= 604800, "Maximum auction duration is 7 days");
        require!(!min_bid_increment.is_zero(), "Min bid increment must be greater than zero");

        let caller = self.blockchain().get_caller();
        let end_timestamp = self.blockchain().get_block_timestamp() + duration_seconds;

        // Receive NFT in escrow
        let payment = self.call_value().single_esdt();
        let nft_token_id = payment.token_identifier.clone();
        let nft_nonce = payment.token_nonce;
        
        require!(payment.amount == BigUint::from(1u32), "Must send exactly 1 NFT");

        let auction_id = self.auction_counter().get();
        let auction = Auction {
            wine_nft_id,
            nft_token_id,
            nft_nonce,
            seller: caller.clone(),
            starting_price: starting_price.clone(),
            current_bid: starting_price,
            highest_bidder: caller.clone(),
            payment_token,
            end_timestamp,
            active: true,
            min_bid_increment,
            bid_count: 0u32,
        };

        self.auctions(auction_id).set(&auction);
        self.auction_counter().set(auction_id + 1);
        self.seller_auctions(&caller).push(&auction_id);
        
        // Update stats
        self.marketplace_stats().update(|stats| {
            stats.active_auctions += 1;
        });

        self.auction_created_event(auction_id, wine_nft_id, &caller, &auction.starting_price);

        auction_id
    }

    // Place bid on auction
    #[payable("*")]
    #[endpoint(placeBid)]
    fn place_bid(&self, auction_id: u32) {
        let mut auction = self.auctions(auction_id).get();
        require!(auction.active, "Auction is not active");
        require!(
            self.blockchain().get_block_timestamp() < auction.end_timestamp,
            "Auction has ended"
        );

        let payment = self.call_value().egld_or_single_esdt();
        require!(
            payment.token_identifier == auction.payment_token,
            "Invalid payment token"
        );

        let bidder = self.blockchain().get_caller();
        require!(bidder != auction.seller, "Cannot bid on your own auction");
        
        let min_bid = &auction.current_bid + &auction.min_bid_increment;
        require!(payment.amount >= min_bid, "Bid too low");

        // Refund previous highest bidder
        if auction.highest_bidder != auction.seller && auction.bid_count > 0 {
            if auction.payment_token.is_egld() {
                self.send().direct_egld(&auction.highest_bidder, &auction.current_bid);
            } else {
                let token_id = auction.payment_token.unwrap_esdt();
                self.send().direct_esdt(&auction.highest_bidder, &token_id, 0, &auction.current_bid);
            }
        }

        // Update auction with new bid
        auction.current_bid = payment.amount.clone();
        auction.highest_bidder = bidder.clone();
        auction.bid_count += 1;
        
        // Extend auction if bid placed in last 10 minutes
        let time_left = auction.end_timestamp - self.blockchain().get_block_timestamp();
        if time_left < 600 { // 10 minutes
            auction.end_timestamp += 600; // Extend by 10 minutes
        }
        
        self.auctions(auction_id).set(&auction);

        self.bid_placed_event(auction_id, &bidder, &payment.amount);
    }

    // Finalize auction
    #[endpoint(finalizeAuction)]
    fn finalize_auction(&self, auction_id: u32) {
        let mut auction = self.auctions(auction_id).get();
        require!(auction.active, "Auction is not active");
        require!(
            self.blockchain().get_block_timestamp() >= auction.end_timestamp,
            "Auction has not ended yet"
        );

        let caller = self.blockchain().get_caller();
        require!(
            caller == auction.seller || caller == auction.highest_bidder,
            "Only seller or highest bidder can finalize"
        );

        auction.active = false;
        self.auctions(auction_id).set(&auction);
        
        // Update stats
        self.marketplace_stats().update(|stats| {
            stats.active_auctions -= 1;
        });

        if auction.bid_count > 0 && auction.highest_bidder != auction.seller {
            // Calculate fees
            let marketplace_fee = &auction.current_bid * self.marketplace_fee_percent().get() / 10000u64;
            let seller_amount = &auction.current_bid - &marketplace_fee;

            // Transfer NFT to winner
            self.send().direct_esdt(
                &auction.highest_bidder,
                &auction.nft_token_id,
                auction.nft_nonce,
                &BigUint::from(1u32),
            );

            // Transfer payment to seller
            if auction.payment_token.is_egld() {
                self.send().direct_egld(&auction.seller, &seller_amount);
            } else {
                let token_id = auction.payment_token.unwrap_esdt();
                self.send().direct_esdt(&auction.seller, &token_id, 0, &seller_amount);
            }

            // Collect marketplace fee
            if !marketplace_fee.is_zero() {
                let owner = self.blockchain().get_owner_address();
                if auction.payment_token.is_egld() {
                    self.send().direct_egld(&owner, &marketplace_fee);
                } else {
                    let token_id = auction.payment_token.unwrap_esdt();
                    self.send().direct_esdt(&owner, &token_id, 0, &marketplace_fee);
                }
            }
            
            // Update sales stats
            self.marketplace_stats().update(|stats| {
                stats.total_sales += 1;
                stats.total_volume += &auction.current_bid;
                stats.total_fees_collected += &marketplace_fee;
            });

            self.auction_finalized_event(
                auction_id,
                &auction.highest_bidder,
                &auction.current_bid
            );
        } else {
            // No bids, return NFT to seller
            self.send().direct_esdt(
                &auction.seller,
                &auction.nft_token_id,
                auction.nft_nonce,
                &BigUint::from(1u32),
            );
            
            self.auction_cancelled_event(auction_id);
        }
    }

    // Cancel listing (only seller, before expiry)
    #[endpoint(cancelListing)]
    fn cancel_listing(&self, listing_id: u32) {
        let mut listing = self.listings(listing_id).get();
        require!(listing.active, "Listing is not active");
        
        let caller = self.blockchain().get_caller();
        require!(caller == listing.seller, "Only seller can cancel listing");
        
        // Return NFT to seller
        self.send().direct_esdt(
            &listing.seller,
            &listing.nft_token_id,
            listing.nft_nonce,
            &BigUint::from(1u32),
        );
        
        listing.active = false;
        self.listings(listing_id).set(&listing);
        
        // Update stats
        self.marketplace_stats().update(|stats| {
            stats.active_listings -= 1;
        });
        
        self.listing_cancelled_event(listing_id);
    }

    // View functions
    #[view(getListing)]
    fn get_listing(&self, listing_id: u32) -> Listing<Self::Api> {
        self.listings(listing_id).get()
    }

    #[view(getAuction)]
    fn get_auction(&self, auction_id: u32) -> Auction<Self::Api> {
        self.auctions(auction_id).get()
    }

    #[view(getMarketplaceStats)]
    fn get_marketplace_stats(&self) -> MarketplaceStats<Self::Api> {
        self.marketplace_stats().get()
    }

    #[view(getMarketplaceFeePercent)]
    fn get_marketplace_fee_percent(&self) -> u64 {
        self.marketplace_fee_percent().get()
    }

    #[view(getSellerListings)]
    fn get_seller_listings(&self, seller: &ManagedAddress) -> ManagedVec<u32> {
        self.seller_listings(seller).get()
    }

    #[view(getSellerAuctions)]
    fn get_seller_auctions(&self, seller: &ManagedAddress) -> ManagedVec<u32> {
        self.seller_auctions(seller).get()
    }

    #[view(isSupportedPaymentToken)]
    fn is_supported_payment_token(&self, token_id: &EgldOrEsdtTokenIdentifier) -> bool {
        self.supported_payment_tokens(token_id).get()
    }

    // Storage mappers
    #[storage_mapper("listings")]
    fn listings(&self, listing_id: u32) -> SingleValueMapper<Listing<Self::Api>>;

    #[storage_mapper("auctions")]
    fn auctions(&self, auction_id: u32) -> SingleValueMapper<Auction<Self::Api>>;

    #[storage_mapper("sellerListings")]
    fn seller_listings(&self, seller: &ManagedAddress) -> VecMapper<u32>;

    #[storage_mapper("sellerAuctions")]
    fn seller_auctions(&self, seller: &ManagedAddress) -> VecMapper<u32>;

    #[storage_mapper("supportedPaymentTokens")]
    fn supported_payment_tokens(&self, token_id: &EgldOrEsdtTokenIdentifier) -> SingleValueMapper<bool>;

    #[storage_mapper("marketplaceStats")]
    fn marketplace_stats(&self) -> SingleValueMapper<MarketplaceStats<Self::Api>>;

    #[storage_mapper("listingCounter")]
    fn listing_counter(&self) -> SingleValueMapper<u32>;

    #[storage_mapper("auctionCounter")]
    fn auction_counter(&self) -> SingleValueMapper<u32>;

    #[storage_mapper("marketplaceFeePercent")]
    fn marketplace_fee_percent(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("wineRegistryAddress")]
    fn wine_registry_address(&self) -> SingleValueMapper<ManagedAddress>;

    // Events
    #[event("wineListed")]
    fn wine_listed_event(
        &self,
        #[indexed] listing_id: u32,
        #[indexed] wine_nft_id: u32,
        #[indexed] seller: &ManagedAddress,
        price: &BigUint,
        payment_token: &EgldOrEsdtTokenIdentifier,
    );

    #[event("wineSold")]
    fn wine_sold_event(
        &self,
        #[indexed] listing_id: u32,
        #[indexed] wine_nft_id: u32,
        #[indexed] seller: &ManagedAddress,
        #[indexed] buyer: &ManagedAddress,
        price: &BigUint,
    );

    #[event("auctionCreated")]
    fn auction_created_event(
        &self,
        #[indexed] auction_id: u32,
        #[indexed] wine_nft_id: u32,
        #[indexed] seller: &ManagedAddress,
        starting_price: &BigUint,
    );

    #[event("bidPlaced")]
    fn bid_placed_event(
        &self,
        #[indexed] auction_id: u32,
        #[indexed] bidder: &ManagedAddress,
        bid_amount: &BigUint,
    );

    #[event("auctionFinalized")]
    fn auction_finalized_event(
        &self,
        #[indexed] auction_id: u32,
        #[indexed] winner: &ManagedAddress,
        final_price: &BigUint,
    );

    #[event("auctionCancelled")]
    fn auction_cancelled_event(
        &self,
        #[indexed] auction_id: u32,
    );

    #[event("listingCancelled")]
    fn listing_cancelled_event(
        &self,
        #[indexed] listing_id: u32,
    );

    #[event("tokenAdded")]
    fn token_added_event(
        &self,
        token_id: &EgldOrEsdtTokenIdentifier,
    );
}