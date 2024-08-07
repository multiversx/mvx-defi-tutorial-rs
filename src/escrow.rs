#![no_std]

use multiversx_sc::derive_imports::*;
use multiversx_sc::imports::*;

#[type_abi]
#[derive(TopEncode, TopDecode)]
pub struct Offer<M: ManagedTypeApi> {
    pub creator: ManagedAddress<M>,
    pub identifier: TokenIdentifier<M>,
    pub nonce: u64,
    pub accepted_token: TokenIdentifier<M>,
    pub accepted_nonce: u64,
    pub accepted_amount: BigUint<M>,
    pub accepted_address: ManagedAddress<M>,
}

#[multiversx_sc::contract]
pub trait Escrow {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[payable("*")]
    #[endpoint(createOffer)]
    fn create_offer(
        &self,
        accepted_token: TokenIdentifier,
        accepted_nonce: u64,
        accepted_amount: BigUint,
        accepted_address: ManagedAddress,
    ) -> u32 {
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();

        require!(
            payment.token_nonce > 0 && payment.amount == 1,
            "ESDT is not an NFT"
        );

        let offer_id = self.last_offer_id().update(|v| {
            *v += 1;

            *v
        });

        let offer = Offer {
            creator: caller,
            identifier: payment.token_identifier,
            nonce: payment.token_nonce,
            accepted_token,
            accepted_nonce,
            accepted_amount,
            accepted_address,
        };

        self.offers(offer_id).set(offer);

        offer_id
    }

    #[endpoint(cancelOffer)]
    fn cancel_offer(&self, offer_id: u32) {
        let offers_mapper = self.offers(offer_id);

        require!(!offers_mapper.is_empty(), "Offer does not exist");

        let caller = self.blockchain().get_caller();

        let offer = offers_mapper.get();

        require!(
            offer.creator == caller,
            "Only the offer creator can cancel it"
        );

        self.created_offers(&caller).swap_remove(&offer_id);
        self.wanted_offers(&offer.accepted_address)
            .swap_remove(&offer_id);

        self.offers(offer_id).clear();

        self.tx()
            .to(&offer.creator)
            .single_esdt(&offer.identifier, offer.nonce, &BigUint::from(1u64))
            .transfer();
    }

    #[payable("*")]
    #[endpoint(acceptOffer)]
    fn accept_offer(&self, offer_id: u32) {
        let offers_mapper = self.offers(offer_id);

        require!(!offers_mapper.is_empty(), "Offer does not exist");

        let offer = offers_mapper.get();
        let payment = self.call_value().single_esdt();

        require!(
            payment.token_identifier == offer.accepted_token
                && payment.token_nonce == offer.accepted_nonce
                && payment.amount == offer.accepted_amount,
            "Incorrect payment for offer"
        );

        self.created_offers(&offer.creator).swap_remove(&offer_id);
        self.wanted_offers(&offer.accepted_address)
            .swap_remove(&offer_id);

        self.offers(offer_id).clear();

        self.tx()
            .to(&offer.creator)
            .single_esdt(
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            )
            .transfer();

        self.tx()
            .to(&offer.accepted_address)
            .single_esdt(&offer.identifier, offer.nonce, &BigUint::from(1u64))
            .transfer();
    }

    #[view(getCreatedOffers)]
    fn get_created_offers(
        &self,
        address: ManagedAddress,
    ) -> MultiValueEncoded<MultiValue2<u32, Offer<Self::Api>>> {
        let mut result = MultiValueEncoded::new();

        for offer_id in self.created_offers(&address).iter() {
            result.push(self.get_offer_result(offer_id));
        }

        result
    }

    #[view(getWantedOffers)]
    fn get_wanted_offers(
        &self,
        address: ManagedAddress,
    ) -> MultiValueEncoded<MultiValue2<u32, Offer<Self::Api>>> {
        let mut result = MultiValueEncoded::new();

        for offer_id in self.wanted_offers(&address).iter() {
            result.push(self.get_offer_result(offer_id));
        }

        result
    }

    fn get_offer_result(&self, offer_id: u32) -> MultiValue2<u32, Offer<Self::Api>> {
        let offer = self.offers(offer_id).get();

        MultiValue2::from((offer_id, offer))
    }

    // storage

    #[view]
    #[storage_mapper("createdOffers")]
    fn created_offers(&self, address: &ManagedAddress) -> UnorderedSetMapper<u32>;

    #[view]
    #[storage_mapper("wantedOffers")]
    fn wanted_offers(&self, address: &ManagedAddress) -> UnorderedSetMapper<u32>;

    #[view]
    #[storage_mapper("offers")]
    fn offers(&self, id: u32) -> SingleValueMapper<Offer<Self::Api>>;

    #[storage_mapper("lastOfferId")]
    fn last_offer_id(&self) -> SingleValueMapper<u32>;
}
