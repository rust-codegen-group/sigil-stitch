public class Mapper<TIn, TOut>
    where TIn : IConvertible
    where TOut : IConvertible, new()
{
    public TOut Map(TIn input) {
        throw new NotImplementedException();
    }
}
