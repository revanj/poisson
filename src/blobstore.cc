#include "rust-renderer/include/blobstore.h"
#include "rust-renderer/external/slang/include/slang.h"

#include <iostream>

BlobstoreClient::BlobstoreClient() {
    SlangSession* session = spCreateSession(nullptr);
    if (session) {
        std::cout << "Slang session created successfully!" << std::endl;
        spDestroySession(session);
    } else {
        std::cerr << "Failed to create Slang session." << std::endl;
    }
    std::cout << "fish" << std::endl;
}

std::unique_ptr<BlobstoreClient> new_blobstore_client() {
  return std::unique_ptr<BlobstoreClient>(new BlobstoreClient());
}