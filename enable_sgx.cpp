#include <stdio.h>
#include "../linux-sgx/common/inc/sgx_capable.h"

int main()
{
    int is_sgx_capable = 0;
    sgx_device_status_t status;

    sgx_is_capable(&is_sgx_capable);
    printf("is_sgx_capable: %d\n", is_sgx_capable);

    sgx_cap_enable_device(&status);
    printf("status: %d\n", (int)status);

    return 0;
}
