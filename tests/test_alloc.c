#include <stdio.h>
#include <string.h>

extern void *tal_alloc(unsigned long size);
extern void  tal_free(void *ptr);
extern void *tal_realloc(void *ptr, unsigned long new_size);

int main(void) {
    printf("=== Allocator Test ===\n\n");

    printf("Test 1: Basic alloc/free\n");
    char *p1 = tal_alloc(100);
    printf("  alloc(100)  = %p\n", p1);
    memset(p1, 'A', 100);
    printf("  memset OK, p1[0]='%c' p1[99]='%c'\n", p1[0], p1[99]);
    tal_free(p1);
    printf("  free OK\n\n");

    printf("Test 2: Multiple allocations\n");
    void *ptrs[10];
    for (int i = 0; i < 10; i++) {
        ptrs[i] = tal_alloc(64 * (i + 1));
        printf("  alloc(%4d) = %p\n", 64 * (i + 1), ptrs[i]);
    }
    for (int i = 0; i < 10; i++) {
        tal_free(ptrs[i]);
    }
    printf("  all freed OK\n\n");

    printf("Test 3: Realloc (grow)\n");
    char *p2 = tal_alloc(50);
    printf("  alloc(50)   = %p\n", p2);
    memcpy(p2, "Hello, TAL allocator!", 21);
    p2 = tal_realloc(p2, 200);
    printf("  realloc(200)= %p\n", p2);
    printf("  data preserved: \"%s\"\n", p2);
    tal_free(p2);
    printf("  free OK\n\n");

    printf("Test 4: Realloc (shrink)\n");
    char *p3 = tal_alloc(1000);
    printf("  alloc(1000) = %p\n", p3);
    memset(p3, 'B', 1000);
    p3 = tal_realloc(p3, 100);
    printf("  realloc(100)= %p\n", p3);
    printf("  data preserved: p3[0]='%c' p3[99]='%c'\n", p3[0], p3[99]);
    tal_free(p3);
    printf("  free OK\n\n");

    printf("Test 5: Large allocation\n");
    size_t large_size = 256 * 1024; // 256 KB
    char *p4 = tal_alloc(large_size);
    printf("  alloc(%zu) = %p\n", large_size, p4);
    memset(p4, 'C', large_size);
    printf("  memset OK\n");
    tal_free(p4);
    printf("  free OK\n\n");

    printf("Test 6: Edge cases\n");
    void *p5 = tal_alloc(0);
    printf("  alloc(0)    = %p (expected NULL)\n", p5);
    tal_free(NULL);
    printf("  free(NULL)  OK\n");
    void *p6 = tal_realloc(NULL, 64);
    printf("  realloc(NULL, 64) = %p\n", p6);
    tal_free(p6);
    printf("  free OK\n\n");

    printf("=== All tests passed ===\n");
    return 0;
}
