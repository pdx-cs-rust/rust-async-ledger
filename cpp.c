#include <stdio.h>

#define HELLO "hello, world"
#define DIFFERENCE(x, y) ((x)-(y))
#define SQUARE(x) ((x)*(x))
#define GOOD_SQUARE(x) (z = (x), z * z)

int z;

int main() {
    char *z = "hello";
    printf("%d\n", GOOD_SQUARE(3));
    return 0;
}
