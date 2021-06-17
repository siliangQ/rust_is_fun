#pragma once
#include <memory>

struct MultiBuf;
class BlobstoreClient
{
public:
	BlobstoreClient();
	// take the argument by reference
	// when the argument is passed, the alias is created
	uint64_t put(MultiBuf &buf) const;
};

std::unique_ptr<BlobstoreClient> new_blobstore_client();