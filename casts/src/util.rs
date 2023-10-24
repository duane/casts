use futures::Future;

pub async fn aretry3<T, E, Fut, F: FnMut() -> Fut>(mut f: F, retries: i32) -> Result<T, E>
where
    Fut: Future<Output = Result<T, E>>,
{
    let mut count = 0;
    loop {
        let result = f().await;

        if result.is_ok() {
            break result;
        } else {
            if count > retries {
                break result;
            }
            count += 1;
        }
    }
}
