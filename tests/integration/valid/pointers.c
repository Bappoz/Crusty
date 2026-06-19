int main(void) {
    int x = 5;
    int *p = &x;
    int *q = p;
    p[0] = x + 1;
    int offset = 1;
    int *r = q + offset;
    int v = r[0];
    return v + p[0] + offset;
}
