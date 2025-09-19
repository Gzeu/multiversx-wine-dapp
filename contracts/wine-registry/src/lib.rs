#![no_std]

use multiversx_sc::derive_imports::*;
use multiversx_sc::imports::*;

// Enhanced Wine Details with additional fields
#[derive(TopEncode, TopDecode, TypeAbi, PartialEq, Debug)]
pub struct WineDetails<M: ManagedTypeApi> {
    pub vineyard: ManagedBuffer<M>,
    pub variety: ManagedBuffer<M>,
    pub vintage_year: u32,
    pub production_date: u64,
    pub quality_grade: u8, // 1-10 scale
    pub alcohol_content: u16, // x100 for precision (e.g., 1250 = 12.50%)
    pub region: ManagedBuffer<M>,
    pub certification: ManagedBuffer<M>,
    pub producer_signature: ManagedBuffer<M>,
    pub ipfs_hash: ManagedBuffer<M>, // IPFS hash for additional metadata
    pub total_bottles: u32,
    pub available_bottles: u32,
    pub price_per_bottle: BigUint<M>,
    pub is_organic: bool,
    pub harvest_date: u64,
    pub aging_process: ManagedBuffer<M>,
    pub tasting_notes: ManagedBuffer<M>,
}

#[derive(TopEncode, TopDecode, TypeAbi, PartialEq, Debug)]
pub struct WineRating<M: ManagedTypeApi> {
    pub rater: ManagedAddress<M>,
    pub rating: u8, // 1-10
    pub review: ManagedBuffer<M>,
    pub timestamp: u64,
}

#[derive(TopEncode, TopDecode, TypeAbi, PartialEq, Debug)]
pub struct CertificationAuthority<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub authority_address: ManagedAddress<M>,
    pub is_active: bool,
    pub certification_count: u32,
}

#[multiversx_sc::contract]
pub trait WineRegistry {
    #[init]
    fn init(&self, registration_fee: BigUint) {
        self.wine_counter().set(1u32);
        self.registration_fee().set(&registration_fee);
        self.total_wines_registered().set(0u32);
        
        // Initialize contract owner as first certification authority
        let owner = self.blockchain().get_owner_address();
        let authority = CertificationAuthority {
            name: ManagedBuffer::from("Contract Owner"),
            authority_address: owner.clone(),
            is_active: true,
            certification_count: 0u32,
        };
        self.certification_authorities(1u32).set(&authority);
        self.authority_counter().set(2u32);
    }

    // Enhanced wine registration with comprehensive data
    #[payable("EGLD")]
    #[endpoint(registerWine)]
    fn register_wine(
        &self,
        vineyard: ManagedBuffer,
        variety: ManagedBuffer,
        vintage_year: u32,
        quality_grade: u8,
        alcohol_content: u16,
        region: ManagedBuffer,
        certification: ManagedBuffer,
        producer_signature: ManagedBuffer,
        ipfs_hash: ManagedBuffer,
        total_bottles: u32,
        price_per_bottle: BigUint,
        is_organic: bool,
        harvest_date: u64,
        aging_process: ManagedBuffer,
        tasting_notes: ManagedBuffer,
    ) -> u32 {
        let payment = self.call_value().egld_value().clone();
        let registration_fee = self.registration_fee().get();
        require!(payment >= registration_fee, "Insufficient registration fee");
        
        // Validation
        require!(quality_grade >= 1 && quality_grade <= 10, "Quality grade must be between 1-10");
        require!(alcohol_content <= 2000, "Alcohol content cannot exceed 20%"); // 2000 = 20.00%
        require!(vintage_year >= 1800 && vintage_year <= 2030, "Invalid vintage year");
        require!(total_bottles > 0, "Total bottles must be greater than 0");
        require!(!price_per_bottle.is_zero(), "Price per bottle must be greater than 0");
        require!(!ipfs_hash.is_empty(), "IPFS hash is required");

        let wine_id = self.wine_counter().get();
        let caller = self.blockchain().get_caller();
        let current_timestamp = self.blockchain().get_block_timestamp();

        let wine_details = WineDetails {
            vineyard,
            variety,
            vintage_year,
            production_date: current_timestamp,
            quality_grade,
            alcohol_content,
            region,
            certification,
            producer_signature,
            ipfs_hash,
            total_bottles,
            available_bottles: total_bottles,
            price_per_bottle,
            is_organic,
            harvest_date,
            aging_process,
            tasting_notes,
        };

        self.wine_details(wine_id).set(&wine_details);
        self.wine_owner(wine_id).set(&caller);
        self.wine_counter().set(wine_id + 1);
        self.total_wines_registered().update(|count| *count += 1);
        
        // Add to producer's wine list
        self.producer_wines(&caller).push(&wine_id);
        
        // Return excess payment
        let excess = &payment - &registration_fee;
        if excess > 0 {
            self.send().direct_egld(&caller, &excess);
        }

        // Emit comprehensive event
        self.wine_registered_event(
            wine_id,
            &caller,
            &wine_details.vineyard,
            &wine_details.variety,
            vintage_year,
            total_bottles
        );

        wine_id
    }

    // Add wine rating system
    #[endpoint(rateWine)]
    fn rate_wine(&self, wine_id: u32, rating: u8, review: ManagedBuffer) {
        require!(self.wine_details(wine_id).is_empty() == false, "Wine does not exist");
        require!(rating >= 1 && rating <= 10, "Rating must be between 1-10");
        
        let caller = self.blockchain().get_caller();
        let current_timestamp = self.blockchain().get_block_timestamp();
        
        // Check if user already rated this wine
        require!(
            self.wine_user_rating(wine_id, &caller).is_empty(),
            "You have already rated this wine"
        );
        
        let wine_rating = WineRating {
            rater: caller.clone(),
            rating,
            review,
            timestamp: current_timestamp,
        };
        
        self.wine_ratings(wine_id).push(&wine_rating);
        self.wine_user_rating(wine_id, &caller).set(&rating);
        
        // Update average rating
        self.update_wine_average_rating(wine_id);
        
        self.wine_rated_event(wine_id, &caller, rating);
    }

    // Certification authority management
    #[only_owner]
    #[endpoint(addCertificationAuthority)]
    fn add_certification_authority(
        &self,
        name: ManagedBuffer,
        authority_address: ManagedAddress,
    ) -> u32 {
        let authority_id = self.authority_counter().get();
        
        let authority = CertificationAuthority {
            name,
            authority_address,
            is_active: true,
            certification_count: 0u32,
        };
        
        self.certification_authorities(authority_id).set(&authority);
        self.authority_counter().set(authority_id + 1);
        
        self.authority_added_event(authority_id, &authority.authority_address);
        
        authority_id
    }

    // Certify wine by authority
    #[endpoint(certifyWine)]
    fn certify_wine(&self, wine_id: u32, certification_hash: ManagedBuffer) {
        require!(self.wine_details(wine_id).is_empty() == false, "Wine does not exist");
        
        let caller = self.blockchain().get_caller();
        require!(self.is_certification_authority(&caller), "Not a certification authority");
        
        self.wine_certifications(wine_id).push(&certification_hash);
        self.wine_certified_by(wine_id, &caller).set(&true);
        
        // Update authority certification count
        let authority_id = self.get_authority_id(&caller);
        self.certification_authorities(authority_id).update(|authority| {
            authority.certification_count += 1;
        });
        
        self.wine_certified_event(wine_id, &caller, &certification_hash);
    }

    // Update wine availability (for marketplace integration)
    #[endpoint(updateWineAvailability)]
    fn update_wine_availability(&self, wine_id: u32, bottles_sold: u32) {
        let caller = self.blockchain().get_caller();
        require!(
            caller == self.wine_owner(wine_id).get() || 
            self.authorized_marketplace(&caller).get(),
            "Not authorized to update availability"
        );
        
        self.wine_details(wine_id).update(|wine| {
            require!(wine.available_bottles >= bottles_sold, "Insufficient bottles available");
            wine.available_bottles -= bottles_sold;
        });
        
        self.wine_availability_updated_event(wine_id, bottles_sold);
    }

    // Internal helper functions
    #[view(updateWineAverageRating)]
    fn update_wine_average_rating(&self, wine_id: u32) {
        let ratings = self.wine_ratings(wine_id);
        let mut total_rating = 0u32;
        let mut count = 0u32;
        
        for rating in ratings.iter() {
            total_rating += rating.rating as u32;
            count += 1;
        }
        
        if count > 0 {
            let average = (total_rating * 100) / count; // x100 for precision
            self.wine_average_rating(wine_id).set(average as u16);
        }
    }

    #[view(isCertificationAuthority)]
    fn is_certification_authority(&self, address: &ManagedAddress) -> bool {
        let authority_count = self.authority_counter().get();
        for i in 1..authority_count {
            let authority = self.certification_authorities(i).get();
            if authority.authority_address == *address && authority.is_active {
                return true;
            }
        }
        false
    }

    #[view(getAuthorityId)]
    fn get_authority_id(&self, address: &ManagedAddress) -> u32 {
        let authority_count = self.authority_counter().get();
        for i in 1..authority_count {
            let authority = self.certification_authorities(i).get();
            if authority.authority_address == *address {
                return i;
            }
        }
        0
    }

    // Enhanced view functions
    #[view(getWineDetails)]
    fn get_wine_details(&self, wine_id: u32) -> WineDetails<Self::Api> {
        self.wine_details(wine_id).get()
    }

    #[view(getWineOwner)]
    fn get_wine_owner(&self, wine_id: u32) -> ManagedAddress {
        self.wine_owner(wine_id).get()
    }

    #[view(getWineRatings)]
    fn get_wine_ratings(&self, wine_id: u32) -> ManagedVec<WineRating<Self::Api>> {
        self.wine_ratings(wine_id).get()
    }

    #[view(getWineAverageRating)]
    fn get_wine_average_rating(&self, wine_id: u32) -> u16 {
        self.wine_average_rating(wine_id).get()
    }

    #[view(getProducerWines)]
    fn get_producer_wines(&self, producer: &ManagedAddress) -> ManagedVec<u32> {
        self.producer_wines(producer).get()
    }

    #[view(getTotalWinesRegistered)]
    fn get_total_wines_registered(&self) -> u32 {
        self.total_wines_registered().get()
    }

    #[view(getRegistrationFee)]
    fn get_registration_fee(&self) -> BigUint {
        self.registration_fee().get()
    }

    // Storage mappers
    #[storage_mapper("wineDetails")]
    fn wine_details(&self, wine_id: u32) -> SingleValueMapper<WineDetails<Self::Api>>;

    #[storage_mapper("wineOwner")]
    fn wine_owner(&self, wine_id: u32) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("wineRatings")]
    fn wine_ratings(&self, wine_id: u32) -> VecMapper<WineRating<Self::Api>>;

    #[storage_mapper("wineUserRating")]
    fn wine_user_rating(&self, wine_id: u32, user: &ManagedAddress) -> SingleValueMapper<u8>;

    #[storage_mapper("wineAverageRating")]
    fn wine_average_rating(&self, wine_id: u32) -> SingleValueMapper<u16>;

    #[storage_mapper("wineCertifications")]
    fn wine_certifications(&self, wine_id: u32) -> VecMapper<ManagedBuffer>;

    #[storage_mapper("wineCertifiedBy")]
    fn wine_certified_by(&self, wine_id: u32, authority: &ManagedAddress) -> SingleValueMapper<bool>;

    #[storage_mapper("certificationAuthorities")]
    fn certification_authorities(&self, authority_id: u32) -> SingleValueMapper<CertificationAuthority<Self::Api>>;

    #[storage_mapper("producerWines")]
    fn producer_wines(&self, producer: &ManagedAddress) -> VecMapper<u32>;

    #[storage_mapper("authorizedMarketplace")]
    fn authorized_marketplace(&self, marketplace: &ManagedAddress) -> SingleValueMapper<bool>;

    #[storage_mapper("wineCounter")]
    fn wine_counter(&self) -> SingleValueMapper<u32>;

    #[storage_mapper("authorityCounter")]
    fn authority_counter(&self) -> SingleValueMapper<u32>;

    #[storage_mapper("totalWinesRegistered")]
    fn total_wines_registered(&self) -> SingleValueMapper<u32>;

    #[storage_mapper("registrationFee")]
    fn registration_fee(&self) -> SingleValueMapper<BigUint>;

    // Events
    #[event("wineRegistered")]
    fn wine_registered_event(
        &self,
        #[indexed] wine_id: u32,
        #[indexed] owner: &ManagedAddress,
        #[indexed] vineyard: &ManagedBuffer,
        variety: &ManagedBuffer,
        vintage_year: u32,
        total_bottles: u32,
    );

    #[event("wineRated")]
    fn wine_rated_event(
        &self,
        #[indexed] wine_id: u32,
        #[indexed] rater: &ManagedAddress,
        rating: u8,
    );

    #[event("wineCertified")]
    fn wine_certified_event(
        &self,
        #[indexed] wine_id: u32,
        #[indexed] authority: &ManagedAddress,
        certification_hash: &ManagedBuffer,
    );

    #[event("authorityAdded")]
    fn authority_added_event(
        &self,
        #[indexed] authority_id: u32,
        #[indexed] authority_address: &ManagedAddress,
    );

    #[event("wineAvailabilityUpdated")]
    fn wine_availability_updated_event(
        &self,
        #[indexed] wine_id: u32,
        bottles_sold: u32,
    );
}