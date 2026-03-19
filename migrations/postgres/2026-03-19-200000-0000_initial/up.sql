CREATE TABLE site(
        id SERIAL NOT NULL PRIMARY KEY,
        name TEXT NOT NULL
);

CREATE TABLE tag(
        id SERIAL NOT NULL PRIMARY KEY,
        name TEXT NOT NULL
);

CREATE TABLE network(
        id SERIAL NOT NULL PRIMARY KEY,
        site_id INTEGER NOT NULL,
        name TEXT NOT NULL,
        FOREIGN KEY (site_id) REFERENCES site(id) ON DELETE CASCADE
);

CREATE TABLE host(
        id SERIAL NOT NULL PRIMARY KEY,
        site_id INTEGER NOT NULL,
        name TEXT NOT NULL,
        os_type TEXT,
        hostname TEXT,
        FOREIGN KEY (site_id) REFERENCES site(id) ON DELETE CASCADE
);

CREATE TABLE address(
        id SERIAL NOT NULL PRIMARY KEY,
        host_id INTEGER NOT NULL,
        network_id INTEGER NOT NULL,
        ip TEXT NOT NULL,
        ip_family INTEGER NOT NULL,
        netmask INTEGER NOT NULL,
        mac TEXT,
        FOREIGN KEY (host_id) REFERENCES host(id) ON DELETE CASCADE,
        FOREIGN KEY (network_id) REFERENCES network(id) ON DELETE CASCADE
);

CREATE TABLE service(
        id SERIAL NOT NULL PRIMARY KEY,
        site_id INTEGER NOT NULL,
        address_id INTEGER NOT NULL,
        port INTEGER NOT NULL,
        ip_proto_number INTEGER NOT NULL,
        state TEXT NOT NULL,
        name TEXT NOT NULL,
        product TEXT,
        version TEXT,
        extra_info TEXT,
        os_type TEXT,
        device_type TEXT,
        hostname TEXT,
        confidence INTEGER,
        method TEXT,
        service_fp TEXT,
        cpe TEXT,
        rpcnum INTEGER,
        lowver INTEGER,
        highver INTEGER,
        owner TEXT,
        FOREIGN KEY (site_id) REFERENCES site(id) ON DELETE CASCADE,
        FOREIGN KEY (address_id) REFERENCES address(id) ON DELETE CASCADE
);

CREATE TABLE tag_assignment(
        id SERIAL NOT NULL PRIMARY KEY,
        service_id INTEGER,
        address_id INTEGER,
        host_id INTEGER,
        network_id INTEGER,
        tag_id INTEGER NOT NULL,
        FOREIGN KEY (service_id) REFERENCES service(id) ON DELETE CASCADE,
        FOREIGN KEY (address_id) REFERENCES address(id) ON DELETE CASCADE,
        FOREIGN KEY (host_id) REFERENCES host(id) ON DELETE CASCADE,
        FOREIGN KEY (network_id) REFERENCES network(id) ON DELETE CASCADE,
        FOREIGN KEY (tag_id) REFERENCES tag(id) ON DELETE CASCADE
);

CREATE TABLE note(
        id SERIAL NOT NULL PRIMARY KEY,
        text TEXT NOT NULL,
        service_id INTEGER,
        address_id INTEGER,
        host_id INTEGER,
        network_id INTEGER,
        FOREIGN KEY (service_id) REFERENCES service(id) ON DELETE CASCADE,
        FOREIGN KEY (address_id) REFERENCES address(id) ON DELETE CASCADE,
        FOREIGN KEY (host_id) REFERENCES host(id) ON DELETE CASCADE,
        FOREIGN KEY (network_id) REFERENCES network(id) ON DELETE CASCADE
);
