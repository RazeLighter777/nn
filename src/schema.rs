// @generated automatically by Diesel CLI.

diesel::table! {
    address (id) {
        id -> Integer,
        host_id -> Integer,
        network_id -> Integer,
        ip -> Text,
        ip_family -> Integer,
        netmask -> Integer,
        mac -> Nullable<Text>,
    }
}

diesel::table! {
    host (id) {
        id -> Integer,
        site_id -> Integer,
        name -> Text,
        os_type -> Nullable<Text>,
        hostname -> Nullable<Text>,
    }
}

diesel::table! {
    network (id) {
        id -> Integer,
        site_id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    note (id) {
        id -> Integer,
        text -> Text,
        service_id -> Nullable<Integer>,
        address_id -> Nullable<Integer>,
        host_id -> Nullable<Integer>,
        network_id -> Nullable<Integer>,
    }
}

diesel::table! {
    service (id) {
        id -> Integer,
        site_id -> Integer,
        address_id -> Integer,
        port -> Integer,
        ip_proto_number -> Integer,
        state -> Text,
        name -> Text,
        product -> Nullable<Text>,
        version -> Nullable<Text>,
        extra_info -> Nullable<Text>,
        os_type -> Nullable<Text>,
        device_type -> Nullable<Text>,
        hostname -> Nullable<Text>,
        confidence -> Nullable<Integer>,
        method -> Nullable<Text>,
        service_fp -> Nullable<Text>,
        cpe -> Nullable<Text>,
        rpcnum -> Nullable<Integer>,
        lowver -> Nullable<Integer>,
        highver -> Nullable<Integer>,
        owner -> Nullable<Text>,
    }
}

diesel::table! {
    site (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    tag (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    tag_assignment (id) {
        id -> Integer,
        service_id -> Nullable<Integer>,
        address_id -> Nullable<Integer>,
        host_id -> Nullable<Integer>,
        network_id -> Nullable<Integer>,
        tag_id -> Integer,
    }
}

diesel::joinable!(address -> host (host_id));
diesel::joinable!(address -> network (network_id));
diesel::joinable!(host -> site (site_id));
diesel::joinable!(network -> site (site_id));
diesel::joinable!(note -> address (address_id));
diesel::joinable!(note -> host (host_id));
diesel::joinable!(note -> network (network_id));
diesel::joinable!(note -> service (service_id));
diesel::joinable!(service -> address (address_id));
diesel::joinable!(service -> site (site_id));
diesel::joinable!(tag_assignment -> address (address_id));
diesel::joinable!(tag_assignment -> host (host_id));
diesel::joinable!(tag_assignment -> network (network_id));
diesel::joinable!(tag_assignment -> service (service_id));
diesel::joinable!(tag_assignment -> tag (tag_id));

diesel::allow_tables_to_appear_in_same_query!(
    address,
    host,
    network,
    note,
    service,
    site,
    tag,
    tag_assignment,
);
