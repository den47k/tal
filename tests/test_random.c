#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

extern void *tal_alloc(unsigned long size);
extern void  tal_free(void *ptr);
extern void *tal_realloc(void *ptr, unsigned long new_size);

#define MAX_BLOCKS     1000
#define NUM_ITERATIONS 1000
#define MAX_ALLOC_SIZE 65536

typedef struct {
    void    *ptr;
    size_t   size;
    uint32_t checksum;
} BlockRecord;

static BlockRecord blocks[MAX_BLOCKS];

static uint32_t calc_checksum(const void *ptr, size_t size) {
    const uint8_t *data = (const uint8_t *)ptr;
    uint32_t sum = 0;
    for (size_t i = 0; i < size; i++) {
        sum = sum * 31 + data[i];
    }
    return sum;
}

static void fill_random(void *ptr, size_t size) {
    uint8_t *data = (uint8_t *)ptr;
    for (size_t i = 0; i < size; i++) {
        data[i] = (uint8_t)(rand() & 0xFF);
    }
}

static int verify_checksum(const BlockRecord *rec) {
    uint32_t actual = calc_checksum(rec->ptr, rec->size);
    if (actual != rec->checksum) {
        fprintf(stderr, "CHECKSUM MISMATCH: ptr=%p size=%zu expected=0x%08x got=0x%08x\n",
                rec->ptr, rec->size, rec->checksum, actual);
        return 0;
    }
    return 1;
}

static int find_empty_slot(void) {
    int start = rand() % MAX_BLOCKS;
    for (int i = 0; i < MAX_BLOCKS; i++) {
        int idx = (start + i) % MAX_BLOCKS;
        if (blocks[idx].ptr == NULL)
            return idx;
    }
    return -1;
}

static int find_occupied_slot(void) {
    int start = rand() % MAX_BLOCKS;
    for (int i = 0; i < MAX_BLOCKS; i++) {
        int idx = (start + i) % MAX_BLOCKS;
        if (blocks[idx].ptr != NULL)
            return idx;
    }
    return -1;
}

static int do_alloc(void) {
    int slot = find_empty_slot();
    if (slot < 0)
        return 0;

    size_t size = (rand() % MAX_ALLOC_SIZE) + 1;
    void *ptr = tal_alloc(size);
    if (ptr == NULL) {
        fprintf(stderr, "tal_alloc(%zu) returned NULL\n", size);
        return 0;
    }

    fill_random(ptr, size);
    blocks[slot].ptr = ptr;
    blocks[slot].size = size;
    blocks[slot].checksum = calc_checksum(ptr, size);
    return 1;
}

static int do_free(void) {
    int slot = find_occupied_slot();
    if (slot < 0)
        return 0;

    if (!verify_checksum(&blocks[slot]))
        return -1;

    tal_free(blocks[slot].ptr);
    blocks[slot].ptr = NULL;
    blocks[slot].size = 0;
    blocks[slot].checksum = 0;
    return 1;
}

static int do_realloc(void) {
    int slot = find_occupied_slot();
    if (slot < 0)
        return 0;

    if (!verify_checksum(&blocks[slot]))
        return -1;

    size_t old_size = blocks[slot].size;
    size_t new_size = (rand() % MAX_ALLOC_SIZE) + 1;

    void *new_ptr = tal_realloc(blocks[slot].ptr, new_size);
    if (new_ptr == NULL) {
        fprintf(stderr, "tal_realloc(%p, %zu) returned NULL\n", blocks[slot].ptr, new_size);
        return 0;
    }

    if (new_size > old_size) {
        fill_random((uint8_t *)new_ptr + old_size, new_size - old_size);
    }

    blocks[slot].ptr = new_ptr;
    blocks[slot].size = new_size;
    blocks[slot].checksum = calc_checksum(new_ptr, new_size);
    return 1;
}

int main(int argc, char *argv[]) {
    unsigned int seed;
    if (argc > 1) {
        seed = (unsigned int)atoi(argv[1]);
    } else {
        seed = (unsigned int)time(NULL);
    }
    srand(seed);
    printf("=== Random Allocator Test ===\n");
    printf("Seed: %u\n", seed);
    printf("Iterations: %d, Max blocks: %d\n\n", NUM_ITERATIONS, MAX_BLOCKS);

    memset(blocks, 0, sizeof(blocks));

    int alloc_count = 0, free_count = 0, realloc_count = 0;
    int errors = 0;

    for (int i = 0; i < NUM_ITERATIONS; i++) {
        int op = rand() % 100;
        int result;

        if (op < 50) {
            result = do_alloc();
            if (result == 1) alloc_count++;
            if (result == -1) errors++;
        } else if (op < 75) {
            result = do_free();
            if (result == 1) free_count++;
            if (result == -1) errors++;
        } else {
            result = do_realloc();
            if (result == 1) realloc_count++;
            if (result == -1) errors++;
        }

        if (errors > 0) {
            fprintf(stderr, "Error detected at iteration %d, aborting.\n", i);
            return 1;
        }
    }

    /* Final verification and cleanup */
    printf("Final verification...\n");
    int remaining = 0;
    for (int i = 0; i < MAX_BLOCKS; i++) {
        if (blocks[i].ptr != NULL) {
            if (!verify_checksum(&blocks[i])) {
                errors++;
                fprintf(stderr, "Final checksum error at slot %d\n", i);
            }
            tal_free(blocks[i].ptr);
            blocks[i].ptr = NULL;
            remaining++;
        }
    }

    printf("\nSummary:\n");
    printf("Allocations: %d\n", alloc_count);
    printf("Frees: %d\n", free_count);
    printf("Reallocs: %d\n", realloc_count);
    printf("Blocks freed in cleanup: %d\n", remaining);

    if (errors > 0) {
        fprintf(stderr, "\n!!! %d ERRORS DETECTED !!!\n", errors);
        return 1;
    }

    printf("\n=== All random tests passed ===\n");
    return 0;
}
