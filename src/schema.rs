// @generated automatically by Diesel CLI.

diesel::table! {
    service (id) {
        id -> Integer,
        site_id -> Integer,
        address_id -> Integer,
        port -> Integer,
        ip_proto_number -> Integer,
        state -> Text,
        name -> Text,
        product -> Text,
        version -> Text,
        extra_info -> Text,
        os_type -> Text,
        device_type -> Text,
        hostname -> Text,
        confidence -> Integer,
        method -> Text,
        service_fp -> Text,
        cpe -> Text,
        rpcnum -> Integer,
        lowver -> Integer,
        highver -> Integer,
        owner -> Text,
    }
}

// represents an IP address.
diesel::table! {
    address (id) {
        id -> Integer,
        host_id -> Integer,
        ip -> Text,
        mac -> Text,

    }
}

// represents a host, meaning a distinct device on the network
diesel::table! {
    host (id) {
        id -> Integer,
        site_id -> Integer,
        name -> Text,
        os_type -> Text,
        hostname -> Text,
    }
}

// represents a site, meaning a distinct organizations network
diesel::table! {
    site (id) {
        id -> Integer,
        name -> Text,
    }
}

// each service belongs to an address.
diesel::joinable!(service -> address (address_id));
// each service belongs to a site.
diesel::joinable!(service -> site (site_id));
// each address belongs to a host.
diesel::joinable!(address -> host (host_id));
// each host belongs to a site.
diesel::joinable!(host -> site (site_id));