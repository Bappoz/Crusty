struct Point {
    int x;
    int y;
};

struct Point origin;

int main(void) {
    origin.x = 0;
    origin.y = 0;
    origin.x = origin.x + 1;
    return origin.x + origin.y;
}
