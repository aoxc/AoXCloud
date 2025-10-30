use crate::domain::entities::contact::{Address, Contact, ContactGroup, Email, Phone};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailDto {
    pub email: String,
    pub r#type: String,
    pub is_primary: bool,
}

impl From<Email> for EmailDto {
    fn from(email: Email) -> Self {
        Self {
            email: email.email,
            r#type: email.r#type,
            is_primary: email.is_primary,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneDto {
    pub number: String,
    pub r#type: String,
    pub is_primary: bool,
}

impl From<Phone> for PhoneDto {
    fn from(phone: Phone) -> Self {
        Self {
            number: phone.number,
            r#type: phone.r#type,
            is_primary: phone.is_primary,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressDto {
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub r#type: String,
    pub is_primary: bool,
}

impl From<Address> for AddressDto {
    fn from(address: Address) -> Self {
        Self {
            street: address.street,
            city: address.city,
            state: address.state,
            postal_code: address.postal_code,
            country: address.country,
            r#type: address.r#type,
            is_primary: address.is_primary,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactDto {
    pub id: String,
    pub address_book_id: String,
    pub uid: String,
    pub full_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub nickname: Option<String>,
    pub email: Vec<EmailDto>,
    pub phone: Vec<PhoneDto>,
    pub address: Vec<AddressDto>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub photo_url: Option<String>,
    pub birthday: Option<NaiveDate>,
    pub anniversary: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub etag: String,
}

impl Default for ContactDto {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            address_book_id: uuid::Uuid::new_v4().to_string(),
            uid: format!("{}@oxicloud", uuid::Uuid::new_v4()),
            full_name: None,
            first_name: None,
            last_name: None,
            nickname: None,
            email: Vec::new(),
            phone: Vec::new(),
            address: Vec::new(),
            organization: None,
            title: None,
            notes: None,
            photo_url: None,
            birthday: None,
            anniversary: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            etag: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl From<Contact> for ContactDto {
    fn from(contact: Contact) -> Self {
        Self {
            id: contact.id.to_string(),
            address_book_id: contact.address_book_id.to_string(),
            uid: contact.uid,
            full_name: contact.full_name,
            first_name: contact.first_name,
            last_name: contact.last_name,
            nickname: contact.nickname,
            email: contact.email.into_iter().map(EmailDto::from).collect(),
            phone: contact.phone.into_iter().map(PhoneDto::from).collect(),
            address: contact.address.into_iter().map(AddressDto::from).collect(),
            organization: contact.organization,
            title: contact.title,
            notes: contact.notes,
            photo_url: contact.photo_url,
            birthday: contact.birthday,
            anniversary: contact.anniversary,
            created_at: contact.created_at,
            updated_at: contact.updated_at,
            etag: contact.etag,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContactDto {
    pub address_book_id: String,
    pub full_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub nickname: Option<String>,
    pub email: Vec<EmailDto>,
    pub phone: Vec<PhoneDto>,
    pub address: Vec<AddressDto>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub photo_url: Option<String>,
    pub birthday: Option<NaiveDate>,
    pub anniversary: Option<NaiveDate>,
    pub user_id: String, // User creating the contact
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContactDto {
    pub full_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub nickname: Option<String>,
    pub email: Option<Vec<EmailDto>>,
    pub phone: Option<Vec<PhoneDto>>,
    pub address: Option<Vec<AddressDto>>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub photo_url: Option<String>,
    pub birthday: Option<NaiveDate>,
    pub anniversary: Option<NaiveDate>,
    pub user_id: String, // User updating the contact
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContactVCardDto {
    pub address_book_id: String,
    pub vcard: String,
    pub user_id: String, // User creating the contact
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactGroupDto {
    pub id: String,
    pub address_book_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub members_count: Option<i32>,
}

impl From<ContactGroup> for ContactGroupDto {
    fn from(group: ContactGroup) -> Self {
        Self {
            id: group.id.to_string(),
            address_book_id: group.address_book_id.to_string(),
            name: group.name,
            created_at: group.created_at,
            updated_at: group.updated_at,
            members_count: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContactGroupDto {
    pub address_book_id: String,
    pub name: String,
    pub user_id: String, // User creating the group
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContactGroupDto {
    pub name: String,
    pub user_id: String, // User updating the group
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMembershipDto {
    pub group_id: String,
    pub contact_id: String,
}
